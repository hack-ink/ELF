use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

/// Request payload for note listing.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListRequest {
	/// Tenant to list notes from.
	pub tenant_id: String,
	/// Project to list notes from.
	pub project_id: String,
	/// Optional agent filter and required owner for `agent_private`.
	pub agent_id: Option<String>,
	/// Optional scope filter.
	pub scope: Option<String>,
	/// Optional lifecycle status filter.
	pub status: Option<String>,
	/// Optional note-type filter.
	pub r#type: Option<String>,
}

/// One note returned by `list`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListItem {
	/// Note identifier.
	pub note_id: Uuid,
	/// Note type discriminator.
	pub r#type: String,
	/// Optional application-defined key.
	pub key: Option<String>,
	/// Scope key for the note.
	pub scope: String,
	/// Lifecycle status for the note.
	pub status: String,
	/// Note body text.
	pub text: String,
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
	/// Structured source reference metadata.
	pub source_ref: Value,
}

/// Response payload for note listing.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListResponse {
	/// Notes visible to the caller after access filtering.
	pub items: Vec<ListItem>,
}
