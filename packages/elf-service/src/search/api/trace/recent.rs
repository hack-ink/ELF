use crate::search::api::trace::{Deserialize, OffsetDateTime, Serialize, Uuid};

/// Request payload for listing recent traces.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraceRecentListRequest {
	/// Tenant that owns the traces.
	pub tenant_id: String,
	/// Project that owns the traces.
	pub project_id: String,
	/// Agent requesting the list.
	pub agent_id: String,

	/// Maximum number of traces to return.
	pub limit: Option<u32>,

	/// Cursor creation timestamp for pagination.
	pub cursor_created_at: Option<OffsetDateTime>,

	/// Cursor trace identifier for pagination.
	pub cursor_trace_id: Option<Uuid>,

	/// Optional agent filter.
	pub agent_id_filter: Option<String>,

	/// Optional read-profile filter.
	pub read_profile: Option<String>,
	#[serde(with = "crate::time_serde::option")]
	/// Optional lower bound for trace creation time.
	pub created_after: Option<OffsetDateTime>,
	#[serde(with = "crate::time_serde::option")]
	/// Optional upper bound for trace creation time.
	pub created_before: Option<OffsetDateTime>,
}

/// Header row returned by recent-trace listing.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RecentTraceHeader {
	/// Trace identifier.
	pub trace_id: Uuid,
	/// Tenant that owns the trace.
	pub tenant_id: String,
	/// Project that owns the trace.
	pub project_id: String,
	/// Agent that ran the trace.
	pub agent_id: String,
	/// Read profile used for the trace.
	pub read_profile: String,
	/// Search query text.
	pub query: String,
	#[serde(with = "crate::time_serde")]
	/// Trace creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Pagination cursor returned by recent-trace listing.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraceRecentCursor {
	#[serde(with = "crate::time_serde")]
	/// Cursor creation timestamp.
	pub created_at: OffsetDateTime,
	/// Cursor trace identifier.
	pub trace_id: Uuid,
}

/// Response payload for recent-trace listing.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraceRecentListResponse {
	/// Response schema identifier.
	pub schema: String,
	/// Returned trace headers.
	pub traces: Vec<RecentTraceHeader>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Cursor for the next page, when more results remain.
	pub next_cursor: Option<TraceRecentCursor>,
}
