use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request payload for note provenance lookup.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceGetRequest {
	/// Tenant that owns the note.
	pub tenant_id: String,
	/// Project that owns the note.
	pub project_id: String,
	/// Identifier of the note to inspect.
	pub note_id: Uuid,
}

/// Request payload for memory-history lookup.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryHistoryGetRequest {
	/// Tenant that owns the memory.
	pub tenant_id: String,
	/// Project that owns the memory.
	pub project_id: String,
	/// Identifier of the note to inspect.
	pub note_id: Uuid,
}

#[derive(Clone, Debug)]
pub(in crate::provenance) struct ValidatedNoteProvenanceRequest {
	pub(in crate::provenance) tenant_id: String,
	pub(in crate::provenance) project_id: String,
	pub(in crate::provenance) note_id: Uuid,
}
