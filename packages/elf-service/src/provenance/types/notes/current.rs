use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use elf_storage::models::MemoryNote;

/// Current note snapshot returned by provenance APIs.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceNote {
	/// Note identifier.
	pub note_id: Uuid,
	/// Tenant that owns the note.
	pub tenant_id: String,
	/// Project that owns the note.
	pub project_id: String,
	/// Agent that wrote the note.
	pub agent_id: String,
	/// Scope key for the note.
	pub scope: String,
	/// Note type discriminator.
	pub r#type: String,
	/// Optional application-defined key.
	pub key: Option<String>,
	/// Note body text.
	pub text: String,
	/// Importance score.
	pub importance: f32,
	/// Confidence score.
	pub confidence: f32,
	/// Lifecycle status.
	pub status: String,
	#[serde(with = "crate::time_serde")]
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	/// Optional expiry timestamp.
	pub expires_at: Option<OffsetDateTime>,
	/// Structured source reference metadata.
	pub source_ref: Value,
	/// Embedding version associated with the note.
	pub embedding_version: String,
	/// Search hit counter.
	pub hit_count: i64,
	#[serde(with = "crate::time_serde::option")]
	/// Timestamp of the most recent hit.
	pub last_hit_at: Option<OffsetDateTime>,
}
impl From<MemoryNote> for NoteProvenanceNote {
	fn from(note: MemoryNote) -> Self {
		Self {
			note_id: note.note_id,
			tenant_id: note.tenant_id,
			project_id: note.project_id,
			agent_id: note.agent_id,
			scope: note.scope,
			r#type: note.r#type,
			key: note.key,
			text: note.text,
			importance: note.importance,
			confidence: note.confidence,
			status: note.status,
			created_at: note.created_at,
			updated_at: note.updated_at,
			expires_at: note.expires_at,
			source_ref: note.source_ref,
			embedding_version: note.embedding_version,
			hit_count: note.hit_count,
			last_hit_at: note.last_hit_at,
		}
	}
}
