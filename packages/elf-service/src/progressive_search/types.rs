mod session;

pub(super) use session::{
	HitItem, NewSearchSession, SESSION_ABSOLUTE_TTL_HOURS, SESSION_SLIDING_TTL_HOURS,
	SearchSession, SearchSessionItemRecord, SearchSessionRow, SearchSessionizePath,
	SearchSessionizedOutput,
};

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{NoteFetchResponse, PayloadLevel, QueryPlan, SearchTrajectorySummary};

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

/// Search-session mode used by progressive-search APIs.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSessionMode {
	/// Quick-find session without a stored query plan.
	QuickFind,
	/// Planned-search session with a stored query plan.
	PlannedSearch,
}
impl SearchSessionMode {
	pub(super) fn as_str(self) -> &'static str {
		match self {
			Self::QuickFind => "quick_find",
			Self::PlannedSearch => "planned_search",
		}
	}
}

impl FromStr for SearchSessionMode {
	type Err = crate::Error;

	fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
		match value {
			"quick_find" => Ok(Self::QuickFind),
			"planned_search" => Ok(Self::PlannedSearch),
			_ => Err(crate::Error::Storage {
				message: format!("Unknown search session mode: {value}"),
			}),
		}
	}
}

impl From<SearchSessionizePath> for SearchSessionMode {
	fn from(path: SearchSessionizePath) -> Self {
		match path {
			SearchSessionizePath::Quick => Self::QuickFind,
			SearchSessionizePath::Planned => Self::PlannedSearch,
		}
	}
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

/// Request payload for timeline projection of a search session.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchTimelineRequest {
	/// Tenant that owns the session.
	pub tenant_id: String,
	/// Project that owns the session.
	pub project_id: String,
	/// Agent requesting the read.
	pub agent_id: String,
	/// Search session identifier.
	pub search_session_id: Uuid,
	/// Desired payload-detail level.
	pub payload_level: PayloadLevel,
	/// Optional timeline grouping mode.
	pub group_by: Option<String>,
}

/// One timeline bucket for a search session.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchTimelineGroup {
	/// Group key, usually a day string.
	pub date: String,
	/// Items that belong to the group.
	pub items: Vec<SearchIndexItem>,
}

/// Response payload for timeline projection.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchTimelineResponse {
	/// Search session identifier.
	pub search_session_id: Uuid,
	#[serde(with = "crate::time_serde")]
	/// Session expiry timestamp.
	pub expires_at: OffsetDateTime,
	/// Timeline groups.
	pub groups: Vec<SearchTimelineGroup>,
}

/// Request payload for materializing details from a search session.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchDetailsRequest {
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
	/// Requested subset of note identifiers.
	pub note_ids: Vec<Uuid>,
	/// When true, records note-hit metrics for returned details.
	pub record_hits: Option<bool>,
}

/// Per-note error payload for detail materialization.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchDetailsError {
	/// Machine-readable error code.
	pub code: String,
	/// Human-readable error message.
	pub message: String,
}

/// Per-note detail result for a search session.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchDetailsResult {
	/// Requested note identifier.
	pub note_id: Uuid,
	/// Materialized note payload, when loading succeeded.
	pub note: Option<NoteFetchResponse>,
	/// Per-note failure, when loading failed.
	pub error: Option<SearchDetailsError>,
}

/// Response payload for detail materialization.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchDetailsResponse {
	/// Search session identifier.
	pub search_session_id: Uuid,
	#[serde(with = "crate::time_serde")]
	/// Session expiry timestamp.
	pub expires_at: OffsetDateTime,
	/// Per-note results.
	pub results: Vec<SearchDetailsResult>,
}
