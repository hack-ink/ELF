use time::OffsetDateTime;
use uuid::Uuid;

use crate::provenance::{
	history::{classify, summaries},
	types::{
		MemoryHistoryEvent, NoteProvenanceIngestDecision, NoteProvenanceNoteVersion,
		rows::{NoteDerivedProposalRow, NoteProposalReviewRow},
	},
};
use elf_storage::models::MemoryNote;

pub(in crate::provenance) fn version_history_event(
	version: &NoteProvenanceNoteVersion,
	decision: Option<&&NoteProvenanceIngestDecision>,
) -> MemoryHistoryEvent {
	let event_type = classify::version_event_type(version.op.as_str(), version.reason.as_str());
	let related_decision_id = decision.map(|decision| decision.decision_id);
	let details = serde_json::json!({
		"reason": version.reason,
		"prev_snapshot": version.prev_snapshot,
		"new_snapshot": version.new_snapshot,
		"ingest_decision": decision.map(|decision| serde_json::json!({
			"decision_id": decision.decision_id,
			"pipeline": decision.pipeline,
			"base_decision": decision.base_decision,
			"policy_decision": decision.policy_decision,
			"note_op": decision.note_op,
			"reason_code": decision.reason_code,
		})),
	});

	MemoryHistoryEvent {
		event_id: format!("memory_note_versions:{}", version.version_id),
		event_type: event_type.to_string(),
		subject_type: "note".to_string(),
		note_id: version.note_id,
		source_table: "memory_note_versions".to_string(),
		source_id: Some(version.version_id),
		related_note_version_id: Some(version.version_id),
		related_decision_id,
		related_proposal_id: None,
		actor: Some(version.actor.clone()),
		op: Some(version.op.clone()),
		reason_code: None,
		summary: summaries::version_summary(event_type, version.reason.as_str()),
		details,
		ts: version.ts,
	}
}

pub(in crate::provenance) fn decision_history_event(
	note_id: Uuid,
	decision: &NoteProvenanceIngestDecision,
) -> MemoryHistoryEvent {
	let event_type = classify::decision_event_type(decision);
	let details = serde_json::json!({
		"pipeline": decision.pipeline,
		"note_type": decision.note_type,
		"note_key": decision.note_key,
		"base_decision": decision.base_decision,
		"policy_decision": decision.policy_decision,
		"note_op": decision.note_op,
		"details": decision.details,
	});

	MemoryHistoryEvent {
		event_id: format!("memory_ingest_decisions:{}", decision.decision_id),
		event_type: event_type.to_string(),
		subject_type: "note".to_string(),
		note_id,
		source_table: "memory_ingest_decisions".to_string(),
		source_id: Some(decision.decision_id),
		related_note_version_id: decision.note_version_id,
		related_decision_id: Some(decision.decision_id),
		related_proposal_id: None,
		actor: Some(decision.agent_id.clone()),
		op: Some(decision.note_op.clone()),
		reason_code: decision.reason_code.clone(),
		summary: summaries::decision_summary(event_type, decision),
		details,
		ts: decision.ts,
	}
}

pub(in crate::provenance) fn expire_history_event(
	note: &MemoryNote,
	expires_at: OffsetDateTime,
) -> MemoryHistoryEvent {
	MemoryHistoryEvent {
		event_id: format!("memory_notes:{}:expire:{expires_at}", note.note_id),
		event_type: "expire".to_string(),
		subject_type: "note".to_string(),
		note_id: note.note_id,
		source_table: "memory_notes".to_string(),
		source_id: Some(note.note_id),
		related_note_version_id: None,
		related_decision_id: None,
		related_proposal_id: None,
		actor: Some(note.agent_id.clone()),
		op: Some("EXPIRE".to_string()),
		reason_code: None,
		summary: "Note reached its persisted expires_at timestamp.".to_string(),
		details: serde_json::json!({
			"status": note.status,
			"expires_at": expires_at,
		}),
		ts: expires_at,
	}
}

pub(in crate::provenance) fn derived_proposal_history_event(
	note_id: Uuid,
	proposal: NoteDerivedProposalRow,
) -> MemoryHistoryEvent {
	MemoryHistoryEvent {
		event_id: format!("consolidation_proposals:{}", proposal.proposal_id),
		event_type: "derived".to_string(),
		subject_type: "note".to_string(),
		note_id,
		source_table: "consolidation_proposals".to_string(),
		source_id: Some(proposal.proposal_id),
		related_note_version_id: None,
		related_decision_id: None,
		related_proposal_id: Some(proposal.proposal_id),
		actor: Some(proposal.agent_id),
		op: Some(proposal.apply_intent.clone()),
		reason_code: None,
		summary: format!(
			"Derived proposal '{}' was created with review_state '{}'.",
			proposal.proposal_kind, proposal.review_state
		),
		details: serde_json::json!({
			"run_id": proposal.run_id,
			"proposal_kind": proposal.proposal_kind,
			"apply_intent": proposal.apply_intent,
			"review_state": proposal.review_state,
			"source_refs": proposal.source_refs,
			"source_snapshot": proposal.source_snapshot,
			"lineage": proposal.lineage,
			"diff": proposal.diff,
			"confidence": proposal.confidence,
			"target_ref": proposal.target_ref,
			"proposed_payload": proposal.proposed_payload,
		}),
		ts: proposal.created_at,
	}
}

pub(in crate::provenance) fn proposal_review_history_event(
	note_id: Uuid,
	review: NoteProposalReviewRow,
) -> MemoryHistoryEvent {
	let event_type = classify::proposal_review_event_type(review.action.as_str());

	MemoryHistoryEvent {
		event_id: format!("consolidation_proposal_reviews:{}", review.review_id),
		event_type: event_type.to_string(),
		subject_type: "note".to_string(),
		note_id,
		source_table: "consolidation_proposal_reviews".to_string(),
		source_id: Some(review.review_id),
		related_note_version_id: None,
		related_decision_id: None,
		related_proposal_id: Some(review.proposal_id),
		actor: Some(review.reviewer_agent_id),
		op: Some(review.action.clone()),
		reason_code: None,
		summary: format!(
			"Proposal review action '{}' moved '{}' from '{}' to '{}'.",
			review.action, review.proposal_kind, review.from_review_state, review.to_review_state
		),
		details: serde_json::json!({
			"proposal_id": review.proposal_id,
			"run_id": review.run_id,
			"proposal_kind": review.proposal_kind,
			"apply_intent": review.apply_intent,
			"from_review_state": review.from_review_state,
			"to_review_state": review.to_review_state,
			"review_comment": review.review_comment,
			"diff": review.diff,
		}),
		ts: review.created_at,
	}
}

pub(in crate::provenance) fn should_emit_decision_event(
	decision: &NoteProvenanceIngestDecision,
) -> bool {
	if matches!(decision.note_op.as_str(), "NONE" | "REJECTED") {
		return true;
	}

	decision.note_version_id.is_none()
}
