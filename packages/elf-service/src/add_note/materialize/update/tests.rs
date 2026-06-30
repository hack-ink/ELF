use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
	add_note::{materialize::update, types::AddNoteInput},
	structured_fields::StructuredFields,
};
use elf_storage::models::MemoryNote;

fn memory_note(now: OffsetDateTime) -> MemoryNote {
	MemoryNote {
		note_id: Uuid::from_u128(1),
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "agent_private".to_string(),
		r#type: "fact".to_string(),
		key: Some("k".to_string()),
		text: "English text.".to_string(),
		importance: 0.5,
		confidence: 0.9,
		status: "active".to_string(),
		created_at: now,
		updated_at: now,
		expires_at: Some(now + Duration::days(7)),
		embedding_version: "v1".to_string(),
		source_ref: serde_json::json!({"source": "test"}),
		hit_count: 0,
		last_hit_at: None,
	}
}

fn add_note_input() -> AddNoteInput {
	AddNoteInput {
		r#type: "fact".to_string(),
		key: Some("k".to_string()),
		text: "English text.".to_string(),
		structured: Some(StructuredFields::default()),
		importance: 0.5,
		confidence: 0.9,
		ttl_days: Some(7),
		source_ref: serde_json::json!({"source": "test"}),
		write_policy: None,
	}
}

#[test]
fn unchanged_update_accepts_same_effective_ttl() {
	let now = OffsetDateTime::UNIX_EPOCH;
	let existing = memory_note(now);
	let note = add_note_input();

	assert!(update::note_update_is_unchanged(
		&existing,
		&note,
		Some(now + Duration::days(7)),
		Some(7),
	));
}

#[test]
fn unchanged_update_detects_source_ref_drift() {
	let now = OffsetDateTime::UNIX_EPOCH;
	let existing = memory_note(now);
	let mut note = add_note_input();

	note.source_ref = serde_json::json!({"source": "changed"});

	assert!(!update::note_update_is_unchanged(
		&existing,
		&note,
		Some(now + Duration::days(7)),
		Some(7),
	));
}
