use std::collections::HashSet;

use serde_json;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	access::SharedSpaceGrantKey,
	work_journal::{types::constants::WORK_JOURNAL_PROMOTION_BOUNDARY_SCHEMA_V1, validation},
};
use elf_storage::models::WorkJournalEntry;

#[test]
fn promotion_boundary_flags_journal_only_without_accepted_ref() {
	let boundary = validation::normalize_promotion_boundary(&serde_json::json!({
		"authoritative_memory_allowed": true
	}))
	.expect("boundary should normalize");

	assert_eq!(boundary["schema"], WORK_JOURNAL_PROMOTION_BOUNDARY_SCHEMA_V1);
	assert_eq!(boundary["authoritative_memory_allowed"], false);
	assert_eq!(boundary["promotion_required_for_current_facts"], true);
	assert_eq!(boundary["requested_authoritative_memory_allowed"], true);
}

#[test]
fn promotion_boundary_preserves_memory_ref_without_granting_shape_only_authority() {
	let boundary = validation::normalize_promotion_boundary(&serde_json::json!({
		"accepted_memory_authority_ref": {
			"schema": "elf.memory_record_ref/v1",
			"kind": "note",
			"id": "11111111-1111-1111-1111-111111111111",
			"status": "active"
		}
	}))
	.expect("boundary should normalize");

	assert_eq!(boundary["authoritative_memory_allowed"], false);
	assert_eq!(boundary["promotion_required_for_current_facts"], true);
	assert_eq!(
		boundary["accepted_memory_authority_ref"]["id"],
		serde_json::json!("11111111-1111-1111-1111-111111111111")
	);
}

#[test]
fn promotion_boundary_preserves_dreaming_ref_without_granting_shape_only_authority() {
	let boundary = validation::normalize_promotion_boundary(&serde_json::json!({
		"accepted_dreaming_review_ref": {
			"schema": "elf.dreaming_review_queue/v1",
			"proposal_id": "22222222-2222-4222-8222-222222222222",
			"review_state": "applied"
		}
	}))
	.expect("boundary should normalize");

	assert_eq!(boundary["authoritative_memory_allowed"], false);
	assert_eq!(boundary["promotion_required_for_current_facts"], true);
	assert_eq!(
		boundary["accepted_dreaming_review_ref"]["proposal_id"],
		serde_json::json!("22222222-2222-4222-8222-222222222222")
	);
}

#[test]
fn promotion_boundary_rejects_forged_accepted_refs() {
	let primitive_result = validation::normalize_promotion_boundary(&serde_json::json!({
		"accepted_memory_authority_ref": true
	}));
	let object_result = validation::normalize_promotion_boundary(&serde_json::json!({
		"accepted_memory_authority_ref": {
			"schema": "elf.memory_record_ref/v1",
			"id": "11111111-1111-1111-1111-111111111111"
		}
	}));

	assert!(primitive_result.is_err());
	assert!(object_result.is_err());
}

#[test]
fn source_refs_reject_non_object_items() {
	let result = validation::validate_source_refs(&[serde_json::json!("XY-1117")]);

	assert!(result.is_err());
}

#[test]
fn read_allowed_enforces_private_and_shared_grants() {
	let allowed = vec!["agent_private".to_string(), "project_shared".to_string()];
	let no_grants = HashSet::new();
	let private = journal_row("agent_private", "agent-a", "active");
	let shared = journal_row("project_shared", "agent-a", "active");
	let inactive = journal_row("agent_private", "agent-a", "deleted");

	assert!(validation::work_journal_read_allowed(&private, "agent-a", &allowed, &no_grants));
	assert!(!validation::work_journal_read_allowed(&private, "agent-b", &allowed, &no_grants));
	assert!(!validation::work_journal_read_allowed(&inactive, "agent-a", &allowed, &no_grants));
	assert!(validation::work_journal_read_allowed(&shared, "agent-a", &allowed, &no_grants));
	assert!(!validation::work_journal_read_allowed(&shared, "agent-b", &allowed, &no_grants));

	let mut grants = HashSet::new();

	grants.insert(SharedSpaceGrantKey {
		scope: "project_shared".to_string(),
		space_owner_agent_id: "agent-a".to_string(),
	});

	assert!(validation::work_journal_read_allowed(&shared, "agent-b", &allowed, &grants));

	let private_only = vec!["agent_private".to_string()];

	assert!(!validation::work_journal_read_allowed(&shared, "agent-b", &private_only, &grants));
}

fn journal_row(scope: &str, agent_id: &str, status: &str) -> WorkJournalEntry {
	let now = OffsetDateTime::now_utc();

	WorkJournalEntry {
		entry_id: Uuid::nil(),
		tenant_id: "tenant".to_string(),
		project_id: "project".to_string(),
		agent_id: agent_id.to_string(),
		scope: scope.to_string(),
		session_id: "session".to_string(),
		family: "session_log".to_string(),
		status: status.to_string(),
		title: None,
		body: "body".to_string(),
		source_refs: serde_json::json!([{ "schema": "source_ref/v1" }]),
		explicit_next_steps: serde_json::json!([]),
		inferred_next_steps: serde_json::json!([]),
		rejected_options: serde_json::json!([]),
		promotion_boundary: serde_json::json!({}),
		redaction_audit: serde_json::json!({}),
		created_at: now,
		updated_at: now,
	}
}
