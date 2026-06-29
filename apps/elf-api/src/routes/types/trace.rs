use super::*;

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct TraceRecentListQuery {
	pub(in crate::routes) limit: Option<u32>,
	pub(in crate::routes) cursor_created_at: Option<String>,
	pub(in crate::routes) cursor_trace_id: Option<Uuid>,
	pub(in crate::routes) agent_id: Option<String>,
	pub(in crate::routes) read_profile: Option<String>,
	pub(in crate::routes) created_after: Option<String>,
	pub(in crate::routes) created_before: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct TraceBundleGetQuery {
	pub(in crate::routes) mode: Option<TraceBundleMode>,
	pub(in crate::routes) stage_items_limit: Option<u32>,
	pub(in crate::routes) candidates_limit: Option<u32>,
}
