use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::provenance::types::rows::NoteVersionRow;

/// One version-history row for a note.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceNoteVersion {
	/// Version row identifier.
	pub version_id: Uuid,
	/// Note identifier.
	pub note_id: Uuid,
	/// Operation recorded in the version row.
	pub op: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Snapshot before the operation, when available.
	pub prev_snapshot: Option<Value>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Snapshot after the operation, when available.
	pub new_snapshot: Option<Value>,
	/// Human-readable reason for the change.
	pub reason: String,
	/// Actor that performed the change.
	pub actor: String,
	#[serde(with = "crate::time_serde")]
	/// Version timestamp.
	pub ts: OffsetDateTime,
}
impl From<NoteVersionRow> for NoteProvenanceNoteVersion {
	fn from(row: NoteVersionRow) -> Self {
		Self {
			version_id: row.version_id,
			note_id: row.note_id,
			op: row.op,
			prev_snapshot: row.prev_snapshot,
			new_snapshot: row.new_snapshot,
			reason: row.reason,
			actor: row.actor,
			ts: row.ts,
		}
	}
}
