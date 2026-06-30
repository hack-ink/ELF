use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::provenance::types::rows::NoteIndexingOutboxRow;

/// One indexing-outbox row for a note.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceIndexingOutbox {
	/// Outbox identifier.
	pub outbox_id: Uuid,
	/// Note identifier.
	pub note_id: Uuid,
	/// Requested indexing operation.
	pub op: String,
	/// Embedding version targeted by the job.
	pub embedding_version: String,
	/// Current outbox status.
	pub status: String,
	/// Number of attempts already made.
	pub attempts: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Most recent failure text, if any.
	pub last_error: Option<String>,
	#[serde(with = "crate::time_serde")]
	/// Earliest time the job may be claimed again.
	pub available_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}
impl From<NoteIndexingOutboxRow> for NoteProvenanceIndexingOutbox {
	fn from(row: NoteIndexingOutboxRow) -> Self {
		Self {
			outbox_id: row.outbox_id,
			note_id: row.note_id,
			op: row.op,
			embedding_version: row.embedding_version,
			status: row.status,
			attempts: row.attempts,
			last_error: row.last_error,
			available_at: row.available_at,
			created_at: row.created_at,
			updated_at: row.updated_at,
		}
	}
}
