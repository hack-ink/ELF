use super::{HashMap, LiveCapturePolicy, LoadedJob, Path, Serialize, Uuid, ValueEnum, serde_json};

#[derive(Debug, Serialize)]
pub(crate) struct MaterializationEvidence {
	pub(crate) schema: &'static str,
	pub(crate) adapter_id: String,
	pub(crate) adapter_kind: AdapterKind,
	pub(crate) status: MaterializationStatus,
	pub(crate) fixtures: String,
	pub(crate) generated_fixtures: String,
	pub(crate) command_evidence: Vec<CommandEvidence>,
	pub(crate) jobs: Vec<MaterializedJobEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CommandEvidence {
	pub(crate) label: String,
	pub(crate) status: MaterializationStatus,
	pub(crate) command: String,
	pub(crate) artifact: Option<String>,
	pub(crate) reason: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct MaterializedJobEvidence {
	pub(crate) job_id: String,
	pub(crate) suite: String,
	pub(crate) title: String,
	pub(crate) status: MaterializationStatus,
	pub(crate) query: String,
	pub(crate) evidence_ids: Vec<String>,
	pub(crate) returned_count: usize,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) indexing_latency_ms: Option<f64>,
	pub(crate) latency_ms: f64,
	pub(crate) trace_id: Option<Uuid>,
	pub(crate) failure: Option<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub(crate) source_mappings: Vec<SourceMappingEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) operator_debug: Option<OperatorDebugMaterializationEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) capture: Option<CaptureMaterializationEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) consolidation: Option<ConsolidationMaterializationEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) knowledge: Option<KnowledgeMaterializationEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) temporal_reconciliation: Option<TemporalReconciliationMaterializationEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) dreaming_readback: Option<DreamingReadbackMaterializationEvidence>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct OperatorDebugMaterializationEvidence {
	pub(crate) trace_available: bool,
	pub(crate) replay_command_available: bool,
	pub(crate) candidate_drop_visibility: String,
	pub(crate) repair_action_clarity: String,
	pub(crate) raw_sql_needed: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct CaptureMaterializationEvidence {
	pub(crate) stored_evidence_ids: Vec<String>,
	pub(crate) excluded_evidence_ids: Vec<String>,
	pub(crate) source_ids: Vec<String>,
	pub(crate) write_policy_audit_count: usize,
	pub(crate) write_policy_exclusion_count: usize,
	pub(crate) write_policy_redaction_count: usize,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub(crate) runtime_source_refs: Vec<CaptureRuntimeSourceRefEvidence>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct ConsolidationMaterializationEvidence {
	pub(crate) run_id: Option<Uuid>,
	pub(crate) proposal_ids: Vec<Uuid>,
	pub(crate) source_lineage_count: usize,
	pub(crate) unsupported_claim_flag_count: usize,
	pub(crate) review_event_count: usize,
	pub(crate) review_actions: Vec<String>,
	pub(crate) final_review_states: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct KnowledgeMaterializationEvidence {
	pub(crate) page_ids: Vec<Uuid>,
	pub(crate) search_result_count: usize,
	pub(crate) lint_finding_count: usize,
	pub(crate) stale_source_finding_count: usize,
	pub(crate) unsupported_claim_count: usize,
	pub(crate) citation_count: usize,
	pub(crate) source_ref_count: usize,
	pub(crate) version_diff_available: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct TemporalReconciliationMaterializationEvidence {
	pub(crate) current_winner_evidence_ids: Vec<String>,
	pub(crate) historical_loser_evidence_ids: Vec<String>,
	pub(crate) supersession_rationale_evidence_ids: Vec<String>,
	pub(crate) tombstone_evidence_ids: Vec<String>,
	pub(crate) invalidation_evidence_ids: Vec<String>,
	pub(crate) conflict_candidate_evidence_ids: Vec<String>,
	pub(crate) retrieved_evidence_ids: Vec<String>,
	pub(crate) selected_evidence_ids: Vec<String>,
	pub(crate) absent_evidence_ids: Vec<String>,
	pub(crate) retrieved_but_dropped_evidence_ids: Vec<String>,
	pub(crate) selected_but_not_narrated_evidence_ids: Vec<String>,
	pub(crate) contradicted_by_lifecycle_evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct DreamingReadbackMaterializationEvidence {
	pub(crate) artifact_kind: String,
	pub(crate) runtime_path: String,
	pub(crate) service_list_count: usize,
	pub(crate) trace_id: Option<Uuid>,
	pub(crate) generated_artifact_count: usize,
	pub(crate) selected_source_refs: Vec<String>,
	pub(crate) missing_source_refs: Vec<String>,
	pub(crate) source_mutation_count: usize,
	pub(crate) no_source_mutation_checked: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct CaptureRuntimeSourceRefEvidence {
	pub(crate) evidence_id: String,
	pub(crate) source_ref: serde_json::Value,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CaptureRuntimeEvidence {
	pub(crate) items: Vec<CaptureRuntimeEvidenceItem>,
}
impl CaptureRuntimeEvidence {
	pub(crate) fn item_for(&self, evidence_id: &str) -> Option<&CaptureRuntimeEvidenceItem> {
		self.items.iter().find(|item| item.evidence_id == evidence_id)
	}
}

#[derive(Clone, Debug)]
pub(crate) struct CaptureRuntimeEvidenceItem {
	pub(crate) evidence_id: String,
	pub(crate) source_id: Option<String>,
	pub(crate) evidence_binding: Option<String>,
	pub(crate) write_policy_applied: bool,
	pub(crate) capture_action: Option<String>,
	pub(crate) source_ref: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub(crate) struct AdapterResponseOutput {
	pub(crate) adapter_id: String,
	pub(crate) answer: AnswerOutput,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) consolidation: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AnswerOutput {
	pub(crate) content: String,
	pub(crate) evidence_ids: Vec<String>,
	pub(crate) claims: Vec<serde_json::Value>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub(crate) pages: Vec<serde_json::Value>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub(crate) memory_summaries: Vec<serde_json::Value>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub(crate) proactive_briefs: Vec<serde_json::Value>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub(crate) scheduled_tasks: Vec<serde_json::Value>,
	pub(crate) latency_ms: f64,
	pub(crate) cost: CostOutput,
	pub(crate) trace_explainability: TraceExplainabilityOutput,
}

#[derive(Debug, Serialize)]
pub(crate) struct CostOutput {
	pub(crate) currency: String,
	pub(crate) amount: f64,
	pub(crate) input_tokens: u64,
	pub(crate) output_tokens: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct TraceExplainabilityOutput {
	pub(crate) trace_id: Option<String>,
	pub(crate) failure_stage: Option<String>,
	pub(crate) failure_reason: Option<String>,
	pub(crate) stages: Vec<TraceStageOutput>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct TraceStageOutput {
	pub(crate) stage_name: String,
	pub(crate) kept_evidence: Vec<String>,
	pub(crate) dropped_evidence: Vec<String>,
	pub(crate) demoted_evidence: Vec<String>,
	pub(crate) distractor_evidence: Vec<String>,
	pub(crate) notes: String,
}

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

#[derive(Debug)]
pub(crate) struct CorpusText {
	pub(crate) evidence_id: String,
	pub(crate) text: String,
	pub(crate) capture: LiveCapturePolicy,
}

#[derive(Debug, Default)]
pub(crate) struct IngestedCorpus {
	pub(crate) capture: CaptureMaterializationEvidence,
	pub(crate) note_ids_by_evidence: HashMap<String, Vec<Uuid>>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct SourceMappingEvidence {
	pub(crate) source: String,
	pub(crate) evidence_ids: Vec<String>,
	pub(crate) mapping_status: String,
	pub(crate) content_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AdapterKind {
	ElfServiceRuntime,
	QmdCliRuntime,
	LightragApiContextExport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MaterializationStatus {
	Pass,
	WrongResult,
	Blocked,
	Incomplete,
	NotEncoded,
}
