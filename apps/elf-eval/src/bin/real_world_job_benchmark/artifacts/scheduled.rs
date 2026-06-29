use super::{
	super::*,
	memory::{MemorySummaryFreshness, MemorySummarySourceTrace},
	proactive::ProactiveSuggestionAction,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ScheduledMemoryTaskArtifact {
	pub(crate) task_run_id: String,
	pub(crate) contract_schema: String,
	pub(crate) generated_at: String,
	pub(crate) scheduled_for: String,
	pub(crate) tenant_id: String,
	pub(crate) project_id: String,
	pub(crate) agent_id: String,
	pub(crate) read_profile: String,
	pub(crate) task_kind: String,
	#[serde(default)]
	pub(crate) outputs: Vec<ScheduledMemoryOutput>,
	pub(crate) source_trace: MemorySummarySourceTrace,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) execution_trace: Option<ScheduledMemoryExecutionTrace>,
	#[serde(default)]
	pub(crate) source_mutations: Vec<Value>,
	#[serde(default)]
	pub(crate) unsupported_claim_flags: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ScheduledMemoryOutput {
	pub(crate) output_id: String,
	pub(crate) output_kind: String,
	pub(crate) text: String,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
	pub(crate) freshness: MemorySummaryFreshness,
	pub(crate) action: ProactiveSuggestionAction,
	#[serde(default)]
	pub(crate) unsupported_claim_flags: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ScheduledMemoryExecutionTrace {
	pub(crate) trace_id: String,
	pub(crate) trigger_kind: String,
	pub(crate) status: String,
	pub(crate) started_at: String,
	pub(crate) completed_at: String,
	pub(crate) output_ref: String,
	#[serde(default)]
	pub(crate) stages: Vec<ScheduledMemoryTraceStage>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ScheduledMemoryTraceStage {
	pub(crate) stage_name: String,
	pub(crate) summary: String,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
}
