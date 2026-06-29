use crate::search::api::trace::{Deserialize, OffsetDateTime, Serialize, Uuid, Value};

/// Search trace metadata persisted for one search run.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchTrace {
	/// Search trace identifier.
	pub trace_id: Uuid,
	/// Tenant that owns the trace.
	pub tenant_id: String,
	/// Project that owns the trace.
	pub project_id: String,
	/// Agent that ran the search.
	pub agent_id: String,
	/// Read profile used for the search.
	pub read_profile: String,
	/// Search query text.
	pub query: String,
	/// Expansion mode label.
	pub expansion_mode: String,
	/// Expanded query strings.
	pub expanded_queries: Vec<String>,
	/// Scopes allowed by the read profile.
	pub allowed_scopes: Vec<String>,
	/// Candidate count observed by the search.
	pub candidate_count: u32,
	/// Top-k budget used by the search.
	pub top_k: u32,
	/// Config snapshot captured for the trace.
	pub config_snapshot: Value,
	#[serde(with = "crate::time_serde")]
	/// Trace creation timestamp.
	pub created_at: OffsetDateTime,
	/// Trace schema version.
	pub trace_version: i32,
}
