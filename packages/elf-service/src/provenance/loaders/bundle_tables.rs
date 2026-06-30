use sqlx::PgPool;
use uuid::Uuid;

use crate::{
	Result,
	provenance::types::{
		NoteProvenanceIndexingOutbox, NoteProvenanceIngestDecision, NoteProvenanceNoteVersion,
		NoteProvenanceRecentTrace,
		constants::{
			NOTE_PROVENANCE_INGEST_DECISIONS_LIMIT, NOTE_PROVENANCE_NOTE_VERSIONS_LIMIT,
			NOTE_PROVENANCE_OUTBOX_LIMIT, NOTE_PROVENANCE_RECENT_TRACES_LIMIT,
		},
		requests::ValidatedNoteProvenanceRequest,
		rows::{NoteIndexingOutboxRow, NoteIngestDecisionRow, NoteRecentTraceRow, NoteVersionRow},
	},
};

pub(in crate::provenance) async fn load_ingest_decisions(
	pool: &PgPool,
	req: &ValidatedNoteProvenanceRequest,
) -> Result<Vec<NoteProvenanceIngestDecision>> {
	let rows: Vec<NoteIngestDecisionRow> = sqlx::query_as::<_, NoteIngestDecisionRow>(
		"\
SELECT
	decision_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	pipeline,
	note_type,
	note_key,
	note_id,
	note_version_id,
	base_decision,
	policy_decision,
	note_op,
	reason_code,
	details,
	ts
FROM memory_ingest_decisions
WHERE note_id = $1 AND tenant_id = $2 AND project_id = $3
ORDER BY ts DESC
LIMIT $4",
	)
	.bind(req.note_id)
	.bind(&req.tenant_id)
	.bind(&req.project_id)
	.bind(NOTE_PROVENANCE_INGEST_DECISIONS_LIMIT)
	.fetch_all(pool)
	.await?;

	Ok(rows.into_iter().map(NoteProvenanceIngestDecision::from).collect())
}

pub(in crate::provenance) async fn load_note_versions(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	note_id: Uuid,
) -> Result<Vec<NoteProvenanceNoteVersion>> {
	let rows: Vec<NoteVersionRow> = sqlx::query_as::<_, NoteVersionRow>(
		"\
SELECT
	memory_note_versions.version_id,
	memory_note_versions.note_id,
	memory_note_versions.op,
	memory_note_versions.prev_snapshot,
	memory_note_versions.new_snapshot,
	memory_note_versions.reason,
	memory_note_versions.actor,
	memory_note_versions.ts
FROM memory_note_versions
JOIN memory_notes n ON n.note_id = memory_note_versions.note_id
WHERE memory_note_versions.note_id = $1
	AND n.tenant_id = $2
	AND n.project_id = $3
ORDER BY memory_note_versions.ts DESC
LIMIT $4",
	)
	.bind(note_id)
	.bind(tenant_id)
	.bind(project_id)
	.bind(NOTE_PROVENANCE_NOTE_VERSIONS_LIMIT)
	.fetch_all(pool)
	.await?;

	Ok(rows.into_iter().map(NoteProvenanceNoteVersion::from).collect())
}

pub(in crate::provenance) async fn load_indexing_outbox(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	note_id: Uuid,
) -> Result<Vec<NoteProvenanceIndexingOutbox>> {
	let rows: Vec<NoteIndexingOutboxRow> = sqlx::query_as::<_, NoteIndexingOutboxRow>(
		"\
SELECT
	indexing_outbox.outbox_id,
	indexing_outbox.note_id,
	indexing_outbox.op,
	indexing_outbox.embedding_version,
	indexing_outbox.status,
	indexing_outbox.attempts,
	indexing_outbox.last_error,
	indexing_outbox.available_at,
	indexing_outbox.created_at,
	indexing_outbox.updated_at
FROM indexing_outbox
JOIN memory_notes n ON n.note_id = indexing_outbox.note_id
WHERE indexing_outbox.note_id = $1
	AND n.tenant_id = $2
	AND n.project_id = $3
ORDER BY indexing_outbox.updated_at DESC
LIMIT $4",
	)
	.bind(note_id)
	.bind(tenant_id)
	.bind(project_id)
	.bind(NOTE_PROVENANCE_OUTBOX_LIMIT)
	.fetch_all(pool)
	.await?;

	Ok(rows.into_iter().map(NoteProvenanceIndexingOutbox::from).collect())
}

pub(in crate::provenance) async fn load_recent_traces_for_note(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	note_id: Uuid,
) -> Result<Vec<NoteProvenanceRecentTrace>> {
	let rows: Vec<NoteRecentTraceRow> = sqlx::query_as::<_, NoteRecentTraceRow>(
		"\
SELECT
	trace_id,
	tenant_id,
	project_id,
	agent_id,
	read_profile,
	query,
	created_at
FROM search_traces
WHERE tenant_id = $1
	AND project_id = $2
	AND trace_id IN (SELECT DISTINCT trace_id FROM search_trace_items WHERE note_id = $3)
ORDER BY created_at DESC, trace_id DESC
LIMIT $4",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(note_id)
	.bind(NOTE_PROVENANCE_RECENT_TRACES_LIMIT)
	.fetch_all(pool)
	.await?;

	Ok(rows.into_iter().map(to_recent_trace).collect())
}

fn to_recent_trace(item: NoteRecentTraceRow) -> NoteProvenanceRecentTrace {
	NoteProvenanceRecentTrace {
		trace_id: item.trace_id,
		tenant_id: item.tenant_id,
		project_id: item.project_id,
		agent_id: item.agent_id,
		read_profile: item.read_profile,
		query: item.query,
		created_at: item.created_at,
	}
}
