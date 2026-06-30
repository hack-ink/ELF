mod cli;
mod consolidation;
mod live;
mod materialization;
mod providers;
mod runtime;

pub(super) use self::{
	cli::{Args, CommandArgs, ElfArgs, LightragArgs, QmdArgs},
	consolidation::{
		LiveConsolidationFixture, LiveConsolidationProposal, PreparedConsolidationRun,
	},
	live::{
		LiveCaptureAction, LiveCapturePolicy, LiveExpectedClaim, LiveJob, LiveMemoryEvolution,
		LoadedJob,
	},
	materialization::{
		AdapterKind, AdapterResponseOutput, AnswerOutput, CaptureMaterializationEvidence,
		CaptureRuntimeEvidence, CaptureRuntimeEvidenceItem, CaptureRuntimeSourceRefEvidence,
		CommandEvidence, ConsolidationMaterializationEvidence, CorpusText, CostOutput,
		DreamingReadbackMaterializationEvidence, DreamingReadbackOutput, IngestedCorpus,
		KnowledgeMaterializationEvidence, MaterializationEvidence, MaterializationStatus,
		MaterializedJob, MaterializedJobEvidence, MaterializedJobInput, MaterializedOutput,
		OperatorDebugMaterializationEvidence, SelectedEvidenceText, SourceMappingEvidence,
		SuiteMaterializationSelection, SuiteMaterializationSelectionInput,
		TemporalReconciliationMaterializationEvidence, TemporalReconciliationSelection,
		TraceExplainabilityOutput, TraceStageOutput,
	},
	providers::{DeterministicEmbedding, NoopExtractor, TokenOverlapRerank},
	runtime::{BaselineRuntime, LightragSource},
};

use crate::{
	BoxFuture, ConsolidationInputRef, ConsolidationProposalInput, Deserialize, EmbeddingProvider,
	EmbeddingProviderConfig, ExtractorProvider, HashMap, LlmProviderConfig, Map, Parser, Path,
	PathBuf, ProviderConfig, RerankProvider, Serialize, Subcommand, Uuid, ValueEnum, embed_text,
	serde_json, terms,
};

pub(super) const JOB_SCHEMA: &str = "elf.real_world_job/v1";
pub(super) const EVIDENCE_SCHEMA: &str = "elf.real_world_live_adapter_materialization/v1";
pub(super) const TENANT_ID: &str = "elf-live-real-world";
pub(super) const AGENT_ID: &str = "elf-live-real-world-agent";
pub(super) const SCOPE: &str = "agent_private";
pub(super) const ELF_NOTE_CHUNK_CHARS: usize = 220;
