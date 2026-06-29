use super::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct OperatorDebugEvidence {
	pub(super) failure_mode: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) trace_id: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) viewer_url: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) admin_trace_bundle_url: Option<String>,
	pub(super) root_cause: String,
	pub(super) steps_to_root_cause: u32,
	pub(super) raw_sql_needed: bool,
	pub(super) dropped_candidate_visibility: String,
	pub(super) trace_completeness: String,
	pub(super) repair_action_clarity: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) trace_available: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) replay_command_available: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) replay_command: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) replay_artifact: Option<String>,
	#[serde(default)]
	pub(super) viewer_panels: Vec<String>,
	#[serde(default)]
	pub(super) cli_steps: Vec<String>,
	#[serde(default)]
	pub(super) trace_evidence: Vec<String>,
	#[serde(default)]
	pub(super) ux_gaps: Vec<OperatorUxGap>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct OperatorUxGap {
	pub(super) gap_id: String,
	pub(super) severity: String,
	pub(super) description: String,
	pub(super) follow_up_issue: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct TraceExplainability {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) trace_id: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) failure_stage: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) failure_reason: Option<String>,
	#[serde(default)]
	pub(super) stages: Vec<TraceStageExplainability>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct TraceStageExplainability {
	pub(super) stage_name: String,
	#[serde(default)]
	pub(super) kept_evidence: Vec<String>,
	#[serde(default)]
	pub(super) dropped_evidence: Vec<String>,
	#[serde(default)]
	pub(super) demoted_evidence: Vec<String>,
	#[serde(default)]
	pub(super) distractor_evidence: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) notes: Option<String>,
}
