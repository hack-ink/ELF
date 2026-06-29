use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::provenance::types::{
	events::MemoryHistoryEvent,
	notes::{
		NoteProvenanceIndexingOutbox, NoteProvenanceIngestDecision, NoteProvenanceNote,
		NoteProvenanceNoteVersion, NoteProvenanceRecentTrace,
	},
};

/// Timeline response for one memory.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryHistoryResponse {
	/// History schema identifier.
	pub schema: String,
	/// Inspected note identifier.
	pub note_id: Uuid,
	/// Chronological memory events.
	pub events: Vec<MemoryHistoryEvent>,
}

/// Full provenance bundle for one note.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceBundleResponse {
	/// Provenance bundle schema identifier.
	pub schema: String,
	/// Current persisted note snapshot.
	pub note: NoteProvenanceNote,
	/// Recorded ingestion decisions for the note.
	pub ingest_decisions: Vec<NoteProvenanceIngestDecision>,
	/// Version-history rows for the note.
	pub note_versions: Vec<NoteProvenanceNoteVersion>,
	/// Indexing outbox history for the note.
	pub indexing_outbox: Vec<NoteProvenanceIndexingOutbox>,
	/// Recent search traces that referenced the note.
	pub recent_traces: Vec<NoteProvenanceRecentTrace>,
	/// Chronological memory event timeline for the note.
	pub history: Vec<MemoryHistoryEvent>,
}
