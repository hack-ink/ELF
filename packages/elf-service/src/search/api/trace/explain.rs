use super::*;

use super::metadata::SearchTrace;

/// Request payload for loading one item-level explanation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchExplainRequest {
	/// Tenant that owns the trace.
	pub tenant_id: String,
	/// Project that owns the trace.
	pub project_id: String,
	/// Agent requesting the explain payload.
	pub agent_id: String,
	/// Result-handle identifier returned by search.
	pub result_handle: Uuid,
}

/// Item-level explain trajectory.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchExplainTrajectory {
	/// Trajectory schema identifier.
	pub schema: String,
	/// Ordered explain stages.
	pub stages: Vec<SearchExplainTrajectoryStage>,
}

/// One stage in an item-level explain trajectory.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchExplainTrajectoryStage {
	/// Zero-based stage order.
	pub stage_order: u32,
	/// Stable stage name.
	pub stage_name: String,
	/// Stage-level payload.
	pub stage_payload: Value,
	/// Per-item metrics.
	pub metrics: Value,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional match information for the selected item.
	pub match_info: Option<SearchExplainTrajectoryMatch>,
}

/// Match reference for one explain trajectory stage.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchExplainTrajectoryMatch {
	/// Match kind label.
	pub kind: String,
	/// Stage-item identifier, when persisted.
	pub item_id: Option<Uuid>,
	/// Note identifier, when applicable.
	pub note_id: Option<Uuid>,
	/// Chunk identifier, when applicable.
	pub chunk_id: Option<Uuid>,
}

/// Explain payload for one ranked search item.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchExplainItem {
	/// Stable result-handle identifier.
	pub result_handle: Uuid,
	/// Note identifier.
	pub note_id: Uuid,
	/// Chunk identifier, when applicable.
	pub chunk_id: Option<Uuid>,
	/// 1-based final rank.
	pub rank: u32,
	/// Item-level explanation payload.
	pub explain: SearchExplain,
}

/// Response payload for item-level explanations.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchExplainResponse {
	/// Trace metadata.
	pub trace: SearchTrace,
	/// Explained item payload.
	pub item: SearchExplainItem,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional explain trajectory.
	pub trajectory: Option<SearchExplainTrajectory>,
}
