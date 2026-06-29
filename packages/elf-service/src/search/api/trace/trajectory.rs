use super::*;

use super::metadata::SearchTrace;

/// Condensed search-trajectory explanation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchTrajectorySummary {
	/// Summary schema identifier.
	pub schema: String,
	/// Ordered summary stages.
	pub stages: Vec<SearchTrajectorySummaryStage>,
}

/// One stage in a condensed search trajectory.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchTrajectorySummaryStage {
	/// Zero-based stage order.
	pub stage_order: u32,
	/// Stable stage name.
	pub stage_name: String,
	/// Number of items after the stage.
	pub item_count: u32,
	/// Free-form stage statistics.
	pub stats: Value,
}

/// One full search-trajectory stage.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchTrajectoryStage {
	/// Zero-based stage order.
	pub stage_order: u32,
	/// Stable stage name.
	pub stage_name: String,
	/// Stage-level payload.
	pub stage_payload: Value,
	/// Item rows for the stage.
	pub items: Vec<SearchTrajectoryStageItem>,
}

/// One item row inside a search-trajectory stage.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchTrajectoryStageItem {
	/// Stage-item identifier, when persisted.
	pub item_id: Option<Uuid>,
	/// Note identifier, when applicable.
	pub note_id: Option<Uuid>,
	/// Chunk identifier, when applicable.
	pub chunk_id: Option<Uuid>,
	/// Free-form per-item metrics.
	pub metrics: Value,
}

/// Full search-trajectory response.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchTrajectoryResponse {
	/// Trace metadata.
	pub trace: SearchTrace,
	/// Condensed trajectory summary.
	pub trajectory: SearchTrajectorySummary,
	/// Full trajectory stages.
	pub stages: Vec<SearchTrajectoryStage>,
}
