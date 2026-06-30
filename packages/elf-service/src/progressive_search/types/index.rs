use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	PayloadLevel, QueryPlan, SearchTrajectorySummary, progressive_search::types::SearchSessionMode,
};

/// Lightweight session-storable search hit used by progressive-search APIs.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchIndexItem {
	/// Note identifier.
	pub note_id: Uuid,
	/// Note type discriminator.
	pub r#type: String,
	/// Optional application-defined key.
	pub key: Option<String>,
	/// Scope key for the note.
	pub scope: String,
	/// Importance score.
	pub importance: f32,
	/// Confidence score.
	pub confidence: f32,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	/// Optional expiry timestamp.
	pub expires_at: Option<OffsetDateTime>,
	/// Final ranked score.
	pub final_score: f32,
	/// Short display summary.
	pub summary: String,
}

/// Response payload for initial indexed search results.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchIndexResponse {
	/// Search trace identifier.
	pub trace_id: Uuid,
	/// Search session identifier used for follow-up requests.
	pub search_session_id: Uuid,
	#[serde(with = "crate::time_serde")]
	/// Session expiry timestamp.
	pub expires_at: OffsetDateTime,
	/// Stored search hits.
	pub items: Vec<SearchIndexItem>,
	/// Optional condensed explain output.
	pub trajectory_summary: Option<SearchTrajectorySummary>,
}

/// Response payload for reloading a stored search session.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchSessionGetResponse {
	/// Search trace identifier.
	pub trace_id: Uuid,
	/// Search session identifier.
	pub search_session_id: Uuid,
	#[serde(with = "crate::time_serde")]
	/// Session expiry timestamp.
	pub expires_at: OffsetDateTime,
	/// Stored hits after trimming to the requested limit.
	pub items: Vec<SearchIndexItem>,
	/// Session mode.
	pub mode: SearchSessionMode,
	/// Stored query plan for planned-search sessions.
	pub query_plan: Option<QueryPlan>,
	/// Optional condensed explain output.
	pub trajectory_summary: Option<SearchTrajectorySummary>,
}

/// Planned-search variant of the indexed search response.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchIndexPlannedResponse {
	/// Search trace identifier.
	pub trace_id: Uuid,
	/// Search session identifier.
	pub search_session_id: Uuid,
	#[serde(with = "crate::time_serde")]
	/// Session expiry timestamp.
	pub expires_at: OffsetDateTime,
	/// Stored hits.
	pub items: Vec<SearchIndexItem>,
	/// Optional condensed explain output.
	pub trajectory_summary: Option<SearchTrajectorySummary>,
	/// Stored query plan for the session.
	pub query_plan: QueryPlan,
}

/// Request payload for reloading a search session.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchSessionGetRequest {
	/// Tenant that owns the session.
	pub tenant_id: String,
	/// Project that owns the session.
	pub project_id: String,
	/// Agent requesting the read.
	pub agent_id: String,
	/// Search session identifier.
	pub search_session_id: Uuid,
	#[serde(default)]
	/// Desired payload-detail level.
	pub payload_level: PayloadLevel,
	/// Optional limit on returned items.
	pub top_k: Option<u32>,
	/// When true, extends the sliding session TTL.
	pub touch: Option<bool>,
}
