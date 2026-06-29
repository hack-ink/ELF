use crate::search::api::trace::{
	Deserialize, OffsetDateTime, Serialize, Uuid,
	explain::SearchExplainItem,
	metadata::SearchTrace,
	replay::TraceReplayCandidate,
	trajectory::{SearchTrajectoryStage, SearchTrajectorySummary},
};

/// Request payload for loading a trace bundle.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraceBundleGetRequest {
	/// Tenant that owns the trace.
	pub tenant_id: String,
	/// Project that owns the trace.
	pub project_id: String,
	/// Agent requesting the bundle.
	pub agent_id: String,
	/// Trace identifier.
	pub trace_id: Uuid,
	#[serde(default)]
	/// Bundle mode controlling output size.
	pub mode: TraceBundleMode,

	/// Optional cap for per-stage items.
	pub stage_items_limit: Option<u32>,

	/// Optional cap for replay candidates.
	pub candidates_limit: Option<u32>,
}

/// Response payload for trace bundles.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraceBundleResponse {
	/// Response schema identifier.
	pub schema: String,
	#[serde(with = "crate::time_serde")]
	/// Bundle generation timestamp.
	pub generated_at: OffsetDateTime,
	/// Trace metadata.
	pub trace: SearchTrace,
	/// Explained items from the trace.
	pub items: Vec<SearchExplainItem>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional condensed trajectory summary.
	pub trajectory_summary: Option<SearchTrajectorySummary>,
	/// Full trajectory stages.
	pub stages: Vec<SearchTrajectoryStage>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional replay candidates.
	pub candidates: Option<Vec<TraceReplayCandidate>>,
}

/// Bundle-size mode for trace exports.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TraceBundleMode {
	#[default]
	/// Return the bounded default export.
	Bounded,
	/// Return the full export.
	Full,
}
