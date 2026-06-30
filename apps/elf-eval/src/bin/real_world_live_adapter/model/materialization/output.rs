use crate::{Serialize, serde_json};

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
