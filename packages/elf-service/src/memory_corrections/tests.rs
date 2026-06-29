use time::OffsetDateTime;
use uuid::Uuid;

use elf_storage::models::MemoryNote;

use super::{
	MemoryCorrectionAction,
	validation::{apply_restore_snapshot, correction_source_ref_for, validate_correction_request},
};

fn note(status: &str) -> MemoryNote {
	MemoryNote {
		note_id: Uuid::new_v4(),
		tenant_id: "tenant".to_string(),
		project_id: "project".to_string(),
		agent_id: "agent".to_string(),
		scope: "agent_private".to_string(),
		r#type: "fact".to_string(),
		key: Some("target".to_string()),
		text: "Fact: Original memory.".to_string(),
		importance: 0.7,
		confidence: 0.9,
		status: status.to_string(),
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		expires_at: None,
		embedding_version: "test:test:4".to_string(),
		source_ref: serde_json::json!({ "schema": "test/source" }),
		hit_count: 0,
		last_hit_at: None,
	}
}

#[test]
fn correction_request_requires_non_empty_reason_and_source() {
	assert!(
		validate_correction_request(
			"tenant",
			"project",
			"actor",
			"because",
			&serde_json::json!({
				"schema": "review"
			})
		)
		.is_ok()
	);
	assert!(
		validate_correction_request(
			"tenant",
			"project",
			"actor",
			"",
			&serde_json::json!({
				"schema": "review"
			})
		)
		.is_err()
	);
	assert!(
		validate_correction_request(
			"tenant",
			"project",
			"actor",
			"because",
			&serde_json::json!({})
		)
		.is_err()
	);
}

#[test]
fn restore_snapshot_must_be_active_and_restores_memory_fields() {
	let snapshot = serde_json::json!({
		"scope": "project_shared",
		"type": "decision",
		"key": null,
		"text": "Decision: Restore the reviewed memory.",
		"importance": 0.8,
		"confidence": 0.95,
		"status": "active",
		"expires_at": null
	});
	let mut note = note("deleted");

	apply_restore_snapshot(&mut note, &snapshot, OffsetDateTime::UNIX_EPOCH)
		.expect("snapshot should restore");

	assert_eq!(note.status, "active");
	assert_eq!(note.scope, "project_shared");
	assert_eq!(note.r#type, "decision");
	assert_eq!(note.key, None);
	assert_eq!(note.text, "Decision: Restore the reviewed memory.");
}

#[test]
fn correction_source_ref_preserves_prior_and_review_evidence() {
	let prior = serde_json::json!({
		"source_ref": { "schema": "prior" },
		"text": "Fact: Prior memory."
	});
	let correction = correction_source_ref_for(
		MemoryCorrectionAction::Supersede,
		&prior,
		&serde_json::json!({ "schema": "review" }),
		"newer source wins",
		"reviewer",
		OffsetDateTime::UNIX_EPOCH,
		None,
	);

	assert_eq!(correction["schema"], "elf.memory_correction/v1");
	assert_eq!(correction["action"], "supersede");
	assert_eq!(correction["prior_source_ref"]["schema"], "prior");
	assert_eq!(correction["correction_source_ref"]["schema"], "review");
}
