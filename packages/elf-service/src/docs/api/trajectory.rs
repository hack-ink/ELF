use serde::Serialize;
use serde_json::Value;

/// Explain payload for a document retrieval run.
#[derive(Clone, Debug, Serialize)]
pub struct DocRetrievalTrajectory {
	/// Trajectory schema identifier.
	pub schema: String,
	/// Ordered retrieval stages.
	pub stages: Vec<DocRetrievalTrajectoryStage>,
}

/// One stage in a document retrieval trajectory.
#[derive(Clone, Debug, Serialize)]
pub struct DocRetrievalTrajectoryStage {
	/// Zero-based stage order.
	pub stage_order: u32,
	/// Stable stage name.
	pub stage_name: String,
	/// Free-form stage statistics.
	pub stats: Value,
}
