use super::{
	loaders::{
		load_indexing_outbox, load_ingest_decisions, load_memory_history_events,
		load_note_versions, load_recent_traces_for_note,
	},
	types::{
		MEMORY_HISTORY_SCHEMA_V1, MemoryHistoryGetRequest, MemoryHistoryResponse,
		NOTE_PROVENANCE_BUNDLE_SCHEMA_V1, NoteProvenanceBundleResponse, NoteProvenanceGetRequest,
		NoteProvenanceNote,
	},
	validation::validate_note_provenance_request,
};
use crate::{ElfService, Error, Result};
use elf_storage::models::MemoryNote;

impl ElfService {
	/// Loads the current note plus recent provenance tables as one bundle.
	pub async fn note_provenance_get(
		&self,
		req: NoteProvenanceGetRequest,
	) -> Result<NoteProvenanceBundleResponse> {
		let req = validate_note_provenance_request(req)?;
		let note = sqlx::query_as::<_, MemoryNote>(
			"\
SELECT *
FROM memory_notes
WHERE note_id = $1
  AND tenant_id = $2
  AND project_id = $3",
		)
		.bind(req.note_id)
		.bind(&req.tenant_id)
		.bind(&req.project_id)
		.fetch_optional(&self.db.pool)
		.await?;
		let Some(note_row) = note else {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		};
		let ingest_decisions = load_ingest_decisions(&self.db.pool, &req).await?;
		let note_versions =
			load_note_versions(&self.db.pool, &req.tenant_id, &req.project_id, req.note_id).await?;
		let indexing_outbox =
			load_indexing_outbox(&self.db.pool, &req.tenant_id, &req.project_id, req.note_id)
				.await?;
		let recent_traces = load_recent_traces_for_note(
			&self.db.pool,
			&req.tenant_id,
			&req.project_id,
			req.note_id,
		)
		.await?;
		let history = load_memory_history_events(&self.db.pool, &req, &note_row).await?;

		Ok(NoteProvenanceBundleResponse {
			schema: NOTE_PROVENANCE_BUNDLE_SCHEMA_V1.to_string(),
			note: NoteProvenanceNote::from(note_row),
			ingest_decisions,
			note_versions,
			indexing_outbox,
			recent_traces,
			history,
		})
	}

	/// Loads the normalized memory-history timeline for one note.
	pub async fn memory_history_get(
		&self,
		req: MemoryHistoryGetRequest,
	) -> Result<MemoryHistoryResponse> {
		let req = validate_note_provenance_request(NoteProvenanceGetRequest {
			tenant_id: req.tenant_id,
			project_id: req.project_id,
			note_id: req.note_id,
		})?;
		let note_row = sqlx::query_as::<_, MemoryNote>(
			"\
SELECT *
FROM memory_notes
WHERE note_id = $1
  AND tenant_id = $2
  AND project_id = $3",
		)
		.bind(req.note_id)
		.bind(&req.tenant_id)
		.bind(&req.project_id)
		.fetch_optional(&self.db.pool)
		.await?;
		let Some(note_row) = note_row else {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		};
		let events = load_memory_history_events(&self.db.pool, &req, &note_row).await?;

		Ok(MemoryHistoryResponse {
			schema: MEMORY_HISTORY_SCHEMA_V1.to_string(),
			note_id: req.note_id,
			events,
		})
	}
}
