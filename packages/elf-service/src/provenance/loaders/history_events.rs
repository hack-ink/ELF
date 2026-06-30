use std::collections::HashMap;

use serde_json::Value;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Result,
	provenance::{
		history::{self},
		loaders::bundle_tables,
		types::{
			MemoryHistoryEvent, NoteProvenanceIngestDecision, NoteProvenanceNoteVersion,
			constants::NOTE_PROVENANCE_HISTORY_LIMIT,
			requests::ValidatedNoteProvenanceRequest,
			rows::{NoteDerivedProposalRow, NoteProposalReviewRow},
		},
	},
};
use elf_storage::models::MemoryNote;

pub(in crate::provenance) async fn load_memory_history_events(
	pool: &PgPool,
	req: &ValidatedNoteProvenanceRequest,
	note: &MemoryNote,
) -> Result<Vec<MemoryHistoryEvent>> {
	let decisions = bundle_tables::load_ingest_decisions(pool, req).await?;
	let versions =
		bundle_tables::load_note_versions(pool, &req.tenant_id, &req.project_id, req.note_id)
			.await?;
	let proposal_ref = serde_json::json!([{ "kind": "note", "id": req.note_id }]);
	let target_ref = serde_json::json!({ "kind": "note", "id": req.note_id });
	let proposals = load_derived_proposals_for_note(pool, req, &proposal_ref, &target_ref).await?;
	let reviews = load_proposal_reviews_for_note(pool, req, &proposal_ref, &target_ref).await?;

	Ok(build_memory_history_events(
		req.note_id,
		note,
		&decisions,
		&versions,
		proposals,
		reviews,
		OffsetDateTime::now_utc(),
	))
}

fn build_memory_history_events(
	note_id: Uuid,
	note: &MemoryNote,
	decisions: &[NoteProvenanceIngestDecision],
	versions: &[NoteProvenanceNoteVersion],
	proposals: Vec<NoteDerivedProposalRow>,
	reviews: Vec<NoteProposalReviewRow>,
	now: OffsetDateTime,
) -> Vec<MemoryHistoryEvent> {
	let mut decision_by_version = HashMap::new();

	for decision in decisions {
		if let Some(version_id) = decision.note_version_id {
			decision_by_version.insert(version_id, decision);
		}
	}

	let mut events = Vec::new();

	for version in versions {
		events.push(history::version_history_event(
			version,
			decision_by_version.get(&version.version_id),
		));
	}
	for decision in decisions {
		if history::should_emit_decision_event(decision) {
			events.push(history::decision_history_event(note_id, decision));
		}
	}

	if let Some(expires_at) = note.expires_at
		&& expires_at <= now
		&& !events.iter().any(|event| event.event_type == "expire")
	{
		events.push(history::expire_history_event(note, expires_at));
	}

	for proposal in proposals {
		events.push(history::derived_proposal_history_event(note_id, proposal));
	}
	for review in reviews {
		events.push(history::proposal_review_history_event(note_id, review));
	}

	events.sort_by(|left, right| {
		left.ts.cmp(&right.ts).then_with(|| left.event_id.cmp(&right.event_id))
	});

	let history_limit = NOTE_PROVENANCE_HISTORY_LIMIT as usize;

	if events.len() > history_limit {
		let drop_count = events.len() - history_limit;

		events.drain(0..drop_count);
	}

	events
}

async fn load_derived_proposals_for_note(
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

async fn load_proposal_reviews_for_note(
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

#[cfg(test)] mod tests;
