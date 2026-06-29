#![allow(clippy::single_component_path_imports, unused_crate_dependencies)]

//! Live adapter materializer for the real-world job benchmark.

use std::{
	collections::{BTreeSet, HashMap},
	env,
	fs::{self, OpenOptions},
	io::Write as _,
	path::{Path, PathBuf},
	process::{Command, Stdio},
	sync::Arc,
	time::Instant,
};

use ::time::{OffsetDateTime, format_description::well_known::Rfc3339};
use blake3::Hasher;
use clap::{Parser, Subcommand, ValueEnum};
use color_eyre::{self, eyre};
use serde::{Deserialize, Serialize};
use serde_json::{self, Map};
use tokio::task::JoinSet;
use uuid::Uuid;

use elf_chunking::ChunkingConfig;
use elf_config::{Config, EmbeddingProviderConfig, LlmProviderConfig, ProviderConfig};
use elf_domain::{
	consolidation::{
		ConsolidationApplyIntent, ConsolidationInputRef, ConsolidationLineage, ConsolidationMarker,
		ConsolidationMarkerSeverity, ConsolidationMarkers, ConsolidationProposalDiff,
		ConsolidationReviewAction, ConsolidationSourceKind, ConsolidationSourceSnapshot,
		ConsolidationUnsupportedClaimFlag,
	},
	knowledge::KnowledgePageKind,
};
use elf_service::{
	AddNoteInput, AddNoteRequest, BoxFuture, ConsolidationProposalInput,
	ConsolidationProposalResponse, ConsolidationProposalReviewRequest,
	ConsolidationProposalsListRequest, ConsolidationRunCreateRequest, ElfService,
	EmbeddingProvider, ExtractorProvider, KnowledgePageLintRequest, KnowledgePageLintResponse,
	KnowledgePageRebuildRequest, KnowledgePageResponse, KnowledgePageSearchRequest, ListRequest,
	PayloadLevel, Providers, RerankProvider, SearchItem, SearchRequest, SearchResponse,
};
use elf_storage::{db::Db, qdrant::QdrantStore};
use elf_testkit::TestDatabase;
use elf_worker::worker::{self, WorkerState};

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

use capture::*;
use consolidation_adapter::*;
use dreaming_readback::*;
use elf_domain_materializers::*;
use elf_runtime::*;
use evidence_selection::*;
use fixtures::*;
use ingestion::*;
use knowledge_adapter::*;
use lightrag::*;
use materialization::*;
use model::*;
use operator_debug::*;
use output::*;
use qmd::*;
use runtime_support::*;
use service_runtime::*;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;

	match Args::parse().command {
		CommandArgs::Elf(args) => run_elf(args).await,
		CommandArgs::Qmd(args) => run_qmd(args),
		CommandArgs::Lightrag(args) => run_lightrag_async(args).await,
	}
}

#[cfg(test)]
#[path = "real_world_live_adapter/tests.rs"]
mod tests;
