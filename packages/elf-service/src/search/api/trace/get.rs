use super::*;

use super::{
	explain::SearchExplainItem, metadata::SearchTrace, trajectory::SearchTrajectorySummary,
};

/// Request payload for loading trace metadata and items.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraceGetRequest {
	/// Tenant that owns the trace.
	pub tenant_id: String,
	/// Project that owns the trace.
	pub project_id: String,
	/// Agent requesting the trace.
	pub agent_id: String,
	/// Trace identifier.
	pub trace_id: Uuid,
}

/// Request payload for loading full trajectory stages.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraceTrajectoryGetRequest {
	/// Tenant that owns the trace.
	pub tenant_id: String,
	/// Project that owns the trace.
	pub project_id: String,
	/// Agent requesting the trajectory.
	pub agent_id: String,
	/// Trace identifier.
	pub trace_id: Uuid,
}

/// Response payload for trace metadata and explained items.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraceGetResponse {
	/// Trace metadata.
	pub trace: SearchTrace,
	/// Explained items from the trace.
	pub items: Vec<SearchExplainItem>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional condensed trajectory summary.
	pub trajectory_summary: Option<SearchTrajectorySummary>,
}
