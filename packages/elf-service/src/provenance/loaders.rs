use std::collections::HashMap;

use serde_json::Value;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{
	history::{
		decision_history_event, derived_proposal_history_event, expire_history_event,
		proposal_review_history_event, should_emit_decision_event, version_history_event,
	},
	types::{
		MemoryHistoryEvent, NOTE_PROVENANCE_HISTORY_LIMIT, NOTE_PROVENANCE_INGEST_DECISIONS_LIMIT,
		NOTE_PROVENANCE_NOTE_VERSIONS_LIMIT, NOTE_PROVENANCE_OUTBOX_LIMIT,
		NOTE_PROVENANCE_RECENT_TRACES_LIMIT, NoteDerivedProposalRow, NoteIndexingOutboxRow,
		NoteIngestDecisionRow, NoteProposalReviewRow, NoteProvenanceIndexingOutbox,
		NoteProvenanceIngestDecision, NoteProvenanceNoteVersion, NoteProvenanceRecentTrace,
		NoteRecentTraceRow, NoteVersionRow, ValidatedNoteProvenanceRequest,
	},
};
use crate::Result;
use elf_storage::models::MemoryNote;

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

pub(super) async fn load_ingest_decisions(
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

pub(super) async fn load_note_versions(
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

pub(super) async fn load_indexing_outbox(
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

pub(super) async fn load_recent_traces_for_note(
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

pub(super) async fn load_memory_history_events(
	pool: &PgPool,
	req: &ValidatedNoteProvenanceRequest,
	note: &MemoryNote,
) -> Result<Vec<MemoryHistoryEvent>> {
	let decisions = load_ingest_decisions(pool, req).await?;
	let versions = load_note_versions(pool, &req.tenant_id, &req.project_id, req.note_id).await?;
	let proposal_ref = serde_json::json!([{ "kind": "note", "id": req.note_id }]);
	let target_ref = serde_json::json!({ "kind": "note", "id": req.note_id });
	let proposals = load_derived_proposals_for_note(pool, req, &proposal_ref, &target_ref).await?;
	let reviews = load_proposal_reviews_for_note(pool, req, &proposal_ref, &target_ref).await?;
	let mut decision_by_version = HashMap::new();

	for decision in &decisions {
		if let Some(version_id) = decision.note_version_id {
			decision_by_version.insert(version_id, decision);
		}
	}

	let mut events = Vec::new();

	for version in &versions {
		events.push(version_history_event(version, decision_by_version.get(&version.version_id)));
	}
	for decision in &decisions {
		if should_emit_decision_event(decision) {
			events.push(decision_history_event(req.note_id, decision));
		}
	}

	if let Some(expires_at) = note.expires_at
		&& expires_at <= OffsetDateTime::now_utc()
		&& !events.iter().any(|event| event.event_type == "expire")
	{
		events.push(expire_history_event(note, expires_at));
	}

	for proposal in proposals {
		events.push(derived_proposal_history_event(req.note_id, proposal));
	}
	for review in reviews {
		events.push(proposal_review_history_event(req.note_id, review));
	}

	events.sort_by(|left, right| {
		left.ts.cmp(&right.ts).then_with(|| left.event_id.cmp(&right.event_id))
	});

	let history_limit = NOTE_PROVENANCE_HISTORY_LIMIT as usize;

	if events.len() > history_limit {
		let drop_count = events.len() - history_limit;

		events.drain(0..drop_count);
	}

	Ok(events)
}

pub(super) async fn load_derived_proposals_for_note(
	pool: &PgPool,
	req: &ValidatedNoteProvenanceRequest,
	proposal_ref: &Value,
	target_ref: &Value,
) -> Result<Vec<NoteDerivedProposalRow>> {
	let rows = sqlx::query_as::<_, NoteDerivedProposalRow>(
		"\
SELECT
	proposal_id,
	run_id,
	agent_id,
	proposal_kind,
	apply_intent,
	review_state,
	source_refs,
	source_snapshot,
	lineage,
	diff,
	confidence,
	COALESCE(target_ref, '{}'::jsonb) AS target_ref,
	COALESCE(proposed_payload, '{}'::jsonb) AS proposed_payload,
	created_at
FROM consolidation_proposals
WHERE tenant_id = $1
	AND project_id = $2
	AND (source_refs @> $3 OR target_ref @> $4)
ORDER BY created_at DESC, proposal_id DESC
LIMIT $5",
	)
	.bind(&req.tenant_id)
	.bind(&req.project_id)
	.bind(proposal_ref)
	.bind(target_ref)
	.bind(NOTE_PROVENANCE_HISTORY_LIMIT)
	.fetch_all(pool)
	.await?;

	Ok(rows)
}

pub(super) async fn load_proposal_reviews_for_note(
	pool: &PgPool,
	req: &ValidatedNoteProvenanceRequest,
	proposal_ref: &Value,
	target_ref: &Value,
) -> Result<Vec<NoteProposalReviewRow>> {
	let rows = sqlx::query_as::<_, NoteProposalReviewRow>(
		"\
SELECT
	reviews.review_id,
	reviews.proposal_id,
	reviews.run_id,
	reviews.reviewer_agent_id,
	reviews.action,
	reviews.from_review_state,
	reviews.to_review_state,
	reviews.review_comment,
	reviews.created_at,
	proposals.proposal_kind,
	proposals.apply_intent,
	proposals.diff
FROM consolidation_proposal_reviews reviews
JOIN consolidation_proposals proposals
	ON proposals.proposal_id = reviews.proposal_id
WHERE reviews.tenant_id = $1
	AND reviews.project_id = $2
	AND (proposals.source_refs @> $3 OR proposals.target_ref @> $4)
ORDER BY reviews.created_at DESC, reviews.review_id DESC
LIMIT $5",
	)
	.bind(&req.tenant_id)
	.bind(&req.project_id)
	.bind(proposal_ref)
	.bind(target_ref)
	.bind(NOTE_PROVENANCE_HISTORY_LIMIT)
	.fetch_all(pool)
	.await?;

	Ok(rows)
}
