use time::OffsetDateTime;
use uuid::Uuid;

use crate::provenance::types::{
	NoteProvenanceIngestDecision, NoteProvenanceNoteVersion,
	constants::NOTE_PROVENANCE_HISTORY_LIMIT,
	rows::{NoteDerivedProposalRow, NoteProposalReviewRow},
};
use elf_storage::models::MemoryNote;

#[test]
fn history_events_link_versions_emit_decisions_and_insert_expiry() {
	let note_id = Uuid::from_u128(1);
	let version_id = Uuid::from_u128(2);
	let decision = ingest_decision(note_id, Uuid::from_u128(3), Some(version_id), "ADD", 20);
	let ignored = ingest_decision(note_id, Uuid::from_u128(4), None, "NONE", 10);
	let version = note_version(note_id, version_id, "ADD", 20);
	let note = memory_note(note_id, Some(ts(30)));
	let events = super::build_memory_history_events(
		note_id,
		&note,
		&[decision, ignored],
		&[version],
		vec![derived_proposal(40)],
		vec![proposal_review(50)],
		ts(60),
	);
	let event_types: Vec<&str> = events.iter().map(|event| event.event_type.as_str()).collect();

	assert_eq!(event_types, vec!["ignore", "add", "expire", "derived", "applied"]);
	assert_eq!(events[1].related_decision_id, Some(Uuid::from_u128(3)));
	assert_eq!(events[1].related_note_version_id, Some(version_id));
	assert_eq!(events[2].source_table, "memory_notes");
	assert_eq!(events[3].source_table, "consolidation_proposals");
	assert_eq!(events[4].source_table, "consolidation_proposal_reviews");
}

#[test]
fn history_events_trim_oldest_events_after_deterministic_sort() {
	let note_id = Uuid::from_u128(11);
	let note = memory_note(note_id, None);
	let decisions: Vec<NoteProvenanceIngestDecision> = (0..=NOTE_PROVENANCE_HISTORY_LIMIT)
		.map(|idx| ingest_decision(note_id, Uuid::from_u128(idx as u128 + 100), None, "NONE", idx))
		.collect();
	let events = super::build_memory_history_events(
		note_id,
		&note,
		&decisions,
		&[],
		Vec::new(),
		Vec::new(),
		ts(NOTE_PROVENANCE_HISTORY_LIMIT + 10),
	);

	assert_eq!(events.len(), NOTE_PROVENANCE_HISTORY_LIMIT as usize);
	assert_eq!(events[0].ts, ts(1));
	assert_eq!(events.last().expect("event").ts, ts(NOTE_PROVENANCE_HISTORY_LIMIT));
}

fn ingest_decision(
	note_id: Uuid,
	decision_id: Uuid,
	note_version_id: Option<Uuid>,
	note_op: &str,
	ts_value: i64,
) -> NoteProvenanceIngestDecision {
	NoteProvenanceIngestDecision {
		decision_id,
		tenant_id: "tenant-a".to_string(),
		project_id: "project-a".to_string(),
		agent_id: "agent-a".to_string(),
		scope: "project".to_string(),
		pipeline: "add_note".to_string(),
		note_type: "fact".to_string(),
		note_key: None,
		note_id: Some(note_id),
		note_version_id,
		base_decision: "remember".to_string(),
		policy_decision: if note_op == "NONE" { "ignore" } else { "remember" }.to_string(),
		note_op: note_op.to_string(),
		reason_code: None,
		details: serde_json::json!({}),
		ts: ts(ts_value),
	}
}

fn note_version(
	note_id: Uuid,
	version_id: Uuid,
	op: &str,
	ts_value: i64,
) -> NoteProvenanceNoteVersion {
	NoteProvenanceNoteVersion {
		version_id,
		note_id,
		op: op.to_string(),
		prev_snapshot: None,
		new_snapshot: Some(serde_json::json!({ "note_id": note_id })),
		reason: "add_note".to_string(),
		actor: "agent-a".to_string(),
		ts: ts(ts_value),
	}
}

fn derived_proposal(ts_value: i64) -> NoteDerivedProposalRow {
	NoteDerivedProposalRow {
		proposal_id: Uuid::from_u128(20),
		run_id: Uuid::from_u128(21),
		agent_id: "agent-a".to_string(),
		proposal_kind: "memory".to_string(),
		apply_intent: "review".to_string(),
		review_state: "pending".to_string(),
		source_refs: serde_json::json!([]),
		source_snapshot: serde_json::json!({}),
		lineage: serde_json::json!({}),
		diff: serde_json::json!({}),
		confidence: 0.9,
		target_ref: serde_json::json!({}),
		proposed_payload: serde_json::json!({}),
		created_at: ts(ts_value),
	}
}

fn proposal_review(ts_value: i64) -> NoteProposalReviewRow {
	NoteProposalReviewRow {
		review_id: Uuid::from_u128(30),
		proposal_id: Uuid::from_u128(20),
		run_id: Uuid::from_u128(21),
		reviewer_agent_id: "reviewer-a".to_string(),
		action: "apply".to_string(),
		from_review_state: "pending".to_string(),
		to_review_state: "applied".to_string(),
		review_comment: Some("ok".to_string()),
		created_at: ts(ts_value),
		proposal_kind: "memory".to_string(),
		apply_intent: "review".to_string(),
		diff: serde_json::json!({}),
	}
}

fn memory_note(note_id: Uuid, expires_at: Option<OffsetDateTime>) -> MemoryNote {
	let created_at = ts(0);

	MemoryNote {
		note_id,
		tenant_id: "tenant-a".to_string(),
		project_id: "project-a".to_string(),
		agent_id: "agent-a".to_string(),
		scope: "project".to_string(),
		r#type: "fact".to_string(),
		key: None,
		text: "A durable English note.".to_string(),
		importance: 0.5,
		confidence: 0.8,
		status: "active".to_string(),
		created_at,
		updated_at: created_at,
		expires_at,
		embedding_version: "v1".to_string(),
		source_ref: serde_json::json!({}),
		hit_count: 0,
		last_hit_at: None,
	}
}

fn ts(value: i64) -> OffsetDateTime {
	OffsetDateTime::from_unix_timestamp(value).expect("valid timestamp")
}
