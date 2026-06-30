use crate::{LoadedJob, Path, Uuid, serde_json};

use super::{
	AdapterKind, AdapterResponseOutput, CaptureMaterializationEvidence, CommandEvidence,
	ConsolidationMaterializationEvidence, DreamingReadbackMaterializationEvidence, IngestedCorpus,
	KnowledgeMaterializationEvidence, MaterializedJobEvidence,
	OperatorDebugMaterializationEvidence, SourceMappingEvidence,
	TemporalReconciliationMaterializationEvidence, TraceStageOutput,
};

#[derive(Debug)]
pub(crate) struct MaterializedJob {
	pub(crate) response: AdapterResponseOutput,
	pub(crate) evidence: MaterializedJobEvidence,
	pub(crate) operator_debug: Option<serde_json::Value>,
}

#[derive(Debug)]
pub(crate) struct MaterializedJobInput {
	pub(crate) content: String,
	pub(crate) evidence_ids: Vec<String>,
	pub(crate) pages: Vec<serde_json::Value>,
	pub(crate) latency_ms: f64,
	pub(crate) indexing_latency_ms: Option<f64>,
	pub(crate) returned_count: usize,
	pub(crate) trace_id: Option<Uuid>,
	pub(crate) failure: Option<String>,
	pub(crate) source_mappings: Vec<SourceMappingEvidence>,
	pub(crate) operator_debug: Option<serde_json::Value>,
	pub(crate) operator_debug_evidence: Option<OperatorDebugMaterializationEvidence>,
	pub(crate) capture: Option<CaptureMaterializationEvidence>,
	pub(crate) capture_failure: Option<String>,
	pub(crate) consolidation_response: Option<serde_json::Value>,
	pub(crate) consolidation: Option<ConsolidationMaterializationEvidence>,
	pub(crate) knowledge: Option<KnowledgeMaterializationEvidence>,
	pub(crate) temporal_reconciliation: Option<TemporalReconciliationMaterializationEvidence>,
	pub(crate) dreaming_readback: Option<DreamingReadbackMaterializationEvidence>,
	pub(crate) memory_summaries: Vec<serde_json::Value>,
	pub(crate) proactive_briefs: Vec<serde_json::Value>,
	pub(crate) scheduled_tasks: Vec<serde_json::Value>,
	pub(crate) trace_stages: Option<Vec<TraceStageOutput>>,
}

#[derive(Debug)]
pub(crate) struct DreamingReadbackOutput {
	pub(crate) content: String,
	pub(crate) evidence_ids: Vec<String>,
	pub(crate) memory_summaries: Vec<serde_json::Value>,
	pub(crate) proactive_briefs: Vec<serde_json::Value>,
	pub(crate) scheduled_tasks: Vec<serde_json::Value>,
	pub(crate) materialization: DreamingReadbackMaterializationEvidence,
	pub(crate) trace_stages: Vec<TraceStageOutput>,
}

#[derive(Debug)]
pub(crate) struct SelectedEvidenceText {
	pub(crate) content: String,
	pub(crate) evidence_ids: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct TemporalReconciliationSelection {
	pub(crate) selected: SelectedEvidenceText,
	pub(crate) evidence: TemporalReconciliationMaterializationEvidence,
	pub(crate) trace_stages: Vec<TraceStageOutput>,
}

pub(crate) struct SuiteMaterializationSelection {
	pub(crate) selected: SelectedEvidenceText,
	pub(crate) trace_stages: Option<Vec<TraceStageOutput>>,
	pub(crate) dreaming_readback: Option<DreamingReadbackMaterializationEvidence>,
	pub(crate) memory_summaries: Vec<serde_json::Value>,
	pub(crate) proactive_briefs: Vec<serde_json::Value>,
	pub(crate) scheduled_tasks: Vec<serde_json::Value>,
}

pub(crate) struct SuiteMaterializationSelectionInput<'a> {
	pub(crate) loaded: &'a LoadedJob,
	pub(crate) ingested: &'a IngestedCorpus,
	pub(crate) capture_failure: &'a Option<String>,
	pub(crate) selected: SelectedEvidenceText,
	pub(crate) trace_stages: Option<Vec<TraceStageOutput>>,
	pub(crate) knowledge: &'a Option<KnowledgeMaterializationEvidence>,
	pub(crate) consolidation: &'a Option<ConsolidationMaterializationEvidence>,
	pub(crate) dreaming_readback: Option<DreamingReadbackOutput>,
}

pub(crate) struct MaterializedOutput<'a> {
	pub(crate) adapter_id: &'a str,
	pub(crate) adapter_kind: AdapterKind,
	pub(crate) fixtures: &'a Path,
	pub(crate) out_fixtures: &'a Path,
	pub(crate) evidence_out: &'a Path,
	pub(crate) jobs: &'a [LoadedJob],
	pub(crate) materialized: &'a [MaterializedJob],
	pub(crate) command_evidence: Vec<CommandEvidence>,
	pub(crate) metadata: Option<serde_json::Value>,
}
