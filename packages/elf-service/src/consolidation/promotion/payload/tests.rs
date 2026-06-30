use std::path::PathBuf;

use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Error;
use elf_config::Config;
use elf_storage::models::ConsolidationProposal;

fn config() -> Config {
	let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../elf-config/tests/fixtures/sample_config.template.toml");

	elf_config::load(path.as_path()).expect("test config should load")
}

fn proposal(target_ref: Value, proposed_payload: Value) -> ConsolidationProposal {
	ConsolidationProposal {
		proposal_id: Uuid::from_u128(1),
		run_id: Uuid::from_u128(2),
		tenant_id: "tenant-a".to_string(),
		project_id: "project-a".to_string(),
		agent_id: "agent-a".to_string(),
		contract_schema: "elf.consolidation.proposal/v1".to_string(),
		proposal_kind: "derived_note".to_string(),
		apply_intent: "create_derived_note".to_string(),
		review_state: "approved".to_string(),
		source_refs: serde_json::json!([{"kind": "note", "id": "source-a"}]),
		source_snapshot: serde_json::json!({"captured": true}),
		lineage: serde_json::json!({"generated_by": "test"}),
		diff: serde_json::json!({}),
		confidence: 0.75,
		unsupported_claim_flags: serde_json::json!([]),
		contradiction_markers: serde_json::json!([]),
		staleness_markers: serde_json::json!([]),
		target_ref,
		proposed_payload,
		reviewer_agent_id: None,
		review_comment: None,
		reviewed_at: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
	}
}

fn valid_payload() -> Value {
	serde_json::json!({
		"type": "fact",
		"text": "Fact: Promotion payloads keep explicit evidence.",
		"source_ref": {"kind": "note", "id": "source-a"},
		"importance": 0.5,
		"confidence": 1.0
	})
}

#[test]
fn decode_promoted_memory_payload_rejects_non_object_source_ref_and_bad_scores() {
	let bad_source = proposal(
		serde_json::json!({}),
		serde_json::json!({
			"type": "fact",
			"text": "Fact: source_ref must stay structured.",
			"source_ref": ["not", "object"]
		}),
	);
	let bad_score = proposal(
		serde_json::json!({}),
		serde_json::json!({
			"type": "fact",
			"text": "Fact: score bounds are enforced.",
			"source_ref": {},
			"importance": 1.5
		}),
	);

	assert!(super::decode_promoted_memory_payload(&bad_source).is_err());
	assert!(super::decode_promoted_memory_payload(&bad_score).is_err());
}

#[test]
fn validate_promoted_memory_payload_maps_writegate_rejections() {
	let payload = super::decode_promoted_memory_payload(&proposal(
		serde_json::json!({}),
		serde_json::json!({
			"type": "fact",
			"text": "",
			"source_ref": {}
		}),
	))
	.expect("payload shape should decode");
	let err = super::validate_promoted_memory_payload(&payload, "agent_private", &config())
		.expect_err("empty text should fail writegate");

	assert!(matches!(err, Error::InvalidRequest { message } if message.contains("REJECT_EMPTY")));
}

#[test]
fn normalized_optional_string_trims_and_drops_empty_values() {
	assert_eq!(
		super::normalized_optional_string(Some("  memory-key  ".to_string())),
		Some("memory-key".to_string())
	);
	assert_eq!(super::normalized_optional_string(Some("   ".to_string())), None);
	assert_eq!(super::normalized_optional_string(None), None);
}

#[test]
fn target_note_id_accepts_id_and_note_id_alias() {
	let note_id = Uuid::from_u128(42);
	let by_id = proposal(serde_json::json!({"id": note_id}), valid_payload());
	let by_note_id = proposal(serde_json::json!({"note_id": note_id}), valid_payload());

	assert_eq!(super::target_note_id(&by_id).expect("id should parse"), note_id);
	assert_eq!(super::target_note_id(&by_note_id).expect("note_id should parse"), note_id);
}

#[test]
fn promotion_refs_preserve_schema_and_review_context() {
	let proposal = proposal(serde_json::json!({}), valid_payload());
	let note_id = Uuid::from_u128(99);
	let target_ref = super::promoted_memory_target_ref(note_id, OffsetDateTime::UNIX_EPOCH);
	let source_ref = super::promotion_source_ref(
		&proposal,
		&serde_json::json!({"kind": "note", "id": "source-a"}),
		"reviewer-a",
		Some("approved"),
		OffsetDateTime::UNIX_EPOCH,
	);

	assert_eq!(target_ref["schema"], "elf.memory_record_ref/v1");
	assert_eq!(target_ref["kind"], "note");
	assert_eq!(target_ref["id"], note_id.to_string());
	assert_eq!(source_ref["schema"], "elf.memory_promotion/v1");
	assert_eq!(source_ref["review"]["action"], "apply");
	assert_eq!(source_ref["review"]["reviewer_agent_id"], "reviewer-a");
	assert_eq!(source_ref["proposed_source_ref"]["kind"], "note");
}
