use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{NoteFetchResponse, PayloadLevel};

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
