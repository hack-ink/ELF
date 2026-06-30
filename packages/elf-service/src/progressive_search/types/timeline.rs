use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{PayloadLevel, progressive_search::types::SearchIndexItem};

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
