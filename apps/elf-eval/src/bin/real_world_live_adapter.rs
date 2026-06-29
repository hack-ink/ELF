#![allow(clippy::single_component_path_imports, unused_crate_dependencies)]

//! Live adapter materializer for the real-world job benchmark.

#[path = "real_world_live_adapter/capture.rs"] mod capture;
#[path = "real_world_live_adapter/consolidation_adapter.rs"] mod consolidation_adapter;
#[path = "real_world_live_adapter/dreaming_readback.rs"] mod dreaming_readback;
#[path = "real_world_live_adapter/elf_domain_materializers.rs"] mod elf_domain_materializers;
#[path = "real_world_live_adapter/elf_runtime.rs"] mod elf_runtime;
#[path = "real_world_live_adapter/evidence_selection.rs"] mod evidence_selection;
#[path = "real_world_live_adapter/fixtures.rs"] mod fixtures;
#[path = "real_world_live_adapter/ingestion.rs"] mod ingestion;
#[path = "real_world_live_adapter/knowledge_adapter.rs"] mod knowledge_adapter;
#[path = "real_world_live_adapter/lightrag.rs"] mod lightrag;
#[path = "real_world_live_adapter/materialization.rs"] mod materialization;
#[path = "real_world_live_adapter/model.rs"] mod model;
#[path = "real_world_live_adapter/operator_debug.rs"] mod operator_debug;
#[path = "real_world_live_adapter/output.rs"] mod output;
#[path = "real_world_live_adapter/qmd.rs"] mod qmd;
#[path = "real_world_live_adapter/runtime_support.rs"] mod runtime_support;
#[path = "real_world_live_adapter/service_runtime.rs"] mod service_runtime;

use std::{
	collections::{BTreeSet, HashMap},
	env, fs,
	path::{Path, PathBuf},
	process::Command,
	sync::Arc,
	time::Instant,
};

use ::time::{OffsetDateTime, format_description::well_known::Rfc3339};
use clap::{Parser, Subcommand, ValueEnum};
use color_eyre::{self, Result, eyre};
use serde::{Deserialize, Serialize};
use serde_json::{self, Map, Value};
use tokio::task::JoinSet;
use uuid::Uuid;

#[cfg(test)] use capture::capture_runtime_evidence_from_source_refs;
use capture::{
	apply_capture_runtime_source_refs, capture_action_str, capture_for_job,
	capture_runtime_evidence_from_search_items, capture_with_runtime_source_refs,
	elf_stored_corpus_texts, validate_capture_runtime_evidence, write_policy_from_value,
};
use consolidation_adapter::{
	consolidation_materialization_evidence, consolidation_review_action,
	live_consolidation_fixture, live_consolidation_response, live_note_ids,
	prepare_consolidation_run, validate_reviewed_consolidation_count,
};
use dreaming_readback::{
	materialize_elf_dreaming_readback, search_response_evidence_ids,
	suite_materialization_selection,
};
use elf_chunking::ChunkingConfig;
use elf_config::{EmbeddingProviderConfig, LlmProviderConfig, ProviderConfig};
use elf_domain::{
	consolidation::{
		ConsolidationApplyIntent, ConsolidationInputRef, ConsolidationLineage, ConsolidationMarker,
		ConsolidationMarkerSeverity, ConsolidationMarkers, ConsolidationProposalDiff,
		ConsolidationReviewAction, ConsolidationSourceKind, ConsolidationSourceSnapshot,
		ConsolidationUnsupportedClaimFlag,
	},
	knowledge::KnowledgePageKind,
};
use elf_domain_materializers::{materialize_elf_consolidation, materialize_elf_knowledge};
use elf_service::{
	AddNoteInput, AddNoteRequest, BoxFuture, ConsolidationProposalInput,
	ConsolidationProposalResponse, ConsolidationProposalReviewRequest,
	ConsolidationProposalsListRequest, ConsolidationRunCreateRequest, ElfService,
	EmbeddingProvider, ExtractorProvider, KnowledgePageLintRequest, KnowledgePageLintResponse,
	KnowledgePageRebuildRequest, KnowledgePageResponse, KnowledgePageSearchRequest, ListRequest,
	PayloadLevel, RerankProvider, SearchItem, SearchRequest, SearchResponse,
};
use elf_storage::{db::Db, qdrant::QdrantStore};
use elf_testkit::TestDatabase;
use elf_worker::worker::{self, WorkerState};
use evidence_selection::{
	answer_claims, elf_selected_evidence_text, expected_claim_text, live_required_evidence_ids,
	required_evidence_satisfied, selected_required_corpus_texts,
};
use fixtures::{corpus_texts, load_jobs, read_dir_paths};
use ingestion::ingest_elf_corpus;
use knowledge_adapter::{
	knowledge_materialization_evidence, knowledge_page_artifact, stale_trap_evidence_ids,
};
use materialization::{
	declared_encoding_job, is_elf_dreaming_readback_live_adapter, materialized_declared_status_job,
	materialized_job, not_encoded_job,
};
#[cfg(test)] use model::LiveCapturePolicy;
use model::{
	AGENT_ID, AdapterKind, AdapterResponseOutput, AnswerOutput, Args, BaselineRuntime,
	CaptureMaterializationEvidence, CaptureRuntimeEvidence, CaptureRuntimeEvidenceItem,
	CaptureRuntimeSourceRefEvidence, CommandArgs, CommandEvidence,
	ConsolidationMaterializationEvidence, CorpusText, CostOutput, DeterministicEmbedding,
	DreamingReadbackMaterializationEvidence, DreamingReadbackOutput, ELF_NOTE_CHUNK_CHARS,
	EVIDENCE_SCHEMA, ElfArgs, IngestedCorpus, JOB_SCHEMA, KnowledgeMaterializationEvidence,
	LightragArgs, LightragSource, LiveCaptureAction, LiveConsolidationFixture,
	LiveConsolidationProposal, LiveExpectedClaim, LiveJob, LiveMemoryEvolution, LoadedJob,
	MaterializationEvidence, MaterializationStatus, MaterializedJob, MaterializedJobEvidence,
	MaterializedJobInput, MaterializedOutput, NoopExtractor, OperatorDebugMaterializationEvidence,
	PreparedConsolidationRun, QmdArgs, SCOPE, SelectedEvidenceText, SourceMappingEvidence,
	SuiteMaterializationSelection, SuiteMaterializationSelectionInput, TENANT_ID,
	TemporalReconciliationMaterializationEvidence, TemporalReconciliationSelection,
	TokenOverlapRerank, TraceExplainabilityOutput, TraceStageOutput,
};
use operator_debug::{elf_replay_command, operator_debug_output, qmd_replay_command};
use output::{aggregate_status, failure_jobs, write_materialized_output};
use runtime_support::{
	deterministic_providers, embed_text, normalize_ascii_alnum_lowercase, note_text_chunks,
	project_id_for_job, push_unique, run_logged_command, run_logged_shell, run_qmd_command,
	runtime_config, short_hash, slug, terms,
};
use service_runtime::{build_service, run_worker};

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	match Args::parse().command {
		CommandArgs::Elf(args) => elf_runtime::run_elf(args).await,
		CommandArgs::Qmd(args) => qmd::run_qmd(args),
		CommandArgs::Lightrag(args) => lightrag::run_lightrag_async(args).await,
	}
}

#[cfg(test)]
#[path = "real_world_live_adapter/tests.rs"]
mod tests;
