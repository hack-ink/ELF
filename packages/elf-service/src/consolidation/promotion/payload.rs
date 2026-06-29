use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Result, access::ORG_PROJECT_ID, consolidation::types::PromotedMemoryPayload};
use elf_config::Config;
use elf_domain::writegate::{self, NoteInput};
use elf_storage::models::ConsolidationProposal;

pub(in crate::consolidation) fn promoted_memory_scope(
	payload: &PromotedMemoryPayload,
	default_scope: &str,
) -> Result<String> {
	match payload.scope.as_deref() {
		Some(raw) => {
			let scope = raw.trim();

			if scope.is_empty() {
				return Err(Error::InvalidRequest {
					message: "proposed_payload.scope must not be empty when provided.".to_string(),
				});
			}

			Ok(scope.to_string())
		},
		None => Ok(default_scope.to_string()),
	}
}

pub(in crate::consolidation) fn promoted_memory_project_id<'a>(
	proposal_project_id: &'a str,
	scope: &str,
) -> &'a str {
	if scope == "org_shared" { ORG_PROJECT_ID } else { proposal_project_id }
}

pub(in crate::consolidation) fn promoted_memory_target_ref(
	note_id: Uuid,
	now: OffsetDateTime,
) -> Value {
	serde_json::json!({
		"schema": "elf.memory_record_ref/v1",
		"kind": "note",
		"id": note_id,
		"status": "active",
		"applied_at": now,
	})
}

pub(super) fn decode_promoted_memory_payload(
	proposal: &ConsolidationProposal,
) -> Result<PromotedMemoryPayload> {
	let payload: PromotedMemoryPayload = serde_json::from_value(proposal.proposed_payload.clone())
		.map_err(|err| Error::InvalidRequest {
			message: format!("proposed_payload is not a memory note payload: {err}"),
		})?;

	if !matches!(payload.source_ref, Value::Object(_)) {
		return Err(Error::InvalidRequest {
			message: "proposed_payload.source_ref must be a JSON object when provided.".to_string(),
		});
	}
	if payload.importance.is_some_and(invalid_score)
		|| payload.confidence.is_some_and(invalid_score)
	{
		return Err(Error::InvalidRequest {
			message: "proposed memory scores must be finite values in 0.0..=1.0.".to_string(),
		});
	}

	Ok(payload)
}

pub(super) fn validate_promoted_memory_payload(
	payload: &PromotedMemoryPayload,
	effective_scope: &str,
	cfg: &Config,
) -> Result<()> {
	let gate = NoteInput {
		note_type: payload.note_type.clone(),
		scope: effective_scope.to_string(),
		text: payload.text.clone(),
	};

	if let Err(code) = writegate::writegate(&gate, cfg) {
		return Err(Error::InvalidRequest {
			message: format!(
				"proposed memory failed writegate: {}",
				crate::writegate_reason_code(code)
			),
		});
	}

	Ok(())
}

pub(super) fn target_note_id(proposal: &ConsolidationProposal) -> Result<Uuid> {
	let raw = proposal
		.target_ref
		.get("id")
		.or_else(|| proposal.target_ref.get("note_id"))
		.and_then(Value::as_str)
		.ok_or_else(|| Error::InvalidRequest {
			message: "update_derived_note requires target_ref.id or target_ref.note_id."
				.to_string(),
		})?;

	Uuid::parse_str(raw).map_err(|err| Error::InvalidRequest {
		message: format!("target_ref note id is invalid: {err}"),
	})
}

pub(super) fn promotion_source_ref(
	proposal: &ConsolidationProposal,
	proposed_source_ref: &Value,
	reviewer_agent_id: &str,
	review_comment: Option<&str>,
	now: OffsetDateTime,
) -> Value {
	serde_json::json!({
		"schema": "elf.memory_promotion/v1",
		"proposal_id": proposal.proposal_id,
		"run_id": proposal.run_id,
		"proposal_kind": proposal.proposal_kind,
		"apply_intent": proposal.apply_intent,
		"source_refs": proposal.source_refs,
		"source_snapshot": proposal.source_snapshot,
		"lineage": proposal.lineage,
		"unsupported_claim_flags": proposal.unsupported_claim_flags,
		"review": {
			"action": "apply",
			"reviewer_agent_id": reviewer_agent_id,
			"review_comment": review_comment,
			"applied_at": now,
		},
		"proposed_source_ref": proposed_source_ref,
	})
}

pub(super) fn normalized_optional_string(value: Option<String>) -> Option<String> {
	value.map(|raw| raw.trim().to_string()).filter(|trimmed| !trimmed.is_empty())
}

fn invalid_score(score: f32) -> bool {
	!score.is_finite() || !(0.0..=1.0).contains(&score)
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use serde_json::Value;
	use time::OffsetDateTime;
	use uuid::Uuid;

	use crate::consolidation::promotion::payload;
	use elf_storage::models::ConsolidationProposal;

	fn config() -> elf_config::Config {
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

		assert!(payload::decode_promoted_memory_payload(&bad_source).is_err());
		assert!(payload::decode_promoted_memory_payload(&bad_score).is_err());
	}

	#[test]
	fn validate_promoted_memory_payload_maps_writegate_rejections() {
		let payload = payload::decode_promoted_memory_payload(&proposal(
			serde_json::json!({}),
			serde_json::json!({
				"type": "fact",
				"text": "",
				"source_ref": {}
			}),
		))
		.expect("payload shape should decode");
		let err = payload::validate_promoted_memory_payload(&payload, "agent_private", &config())
			.expect_err("empty text should fail writegate");

		assert!(
			matches!(err, crate::Error::InvalidRequest { message } if message.contains("REJECT_EMPTY"))
		);
	}

	#[test]
	fn normalized_optional_string_trims_and_drops_empty_values() {
		assert_eq!(
			payload::normalized_optional_string(Some("  memory-key  ".to_string())),
			Some("memory-key".to_string())
		);
		assert_eq!(payload::normalized_optional_string(Some("   ".to_string())), None);
		assert_eq!(payload::normalized_optional_string(None), None);
	}

	#[test]
	fn target_note_id_accepts_id_and_note_id_alias() {
		let note_id = Uuid::from_u128(42);
		let by_id = proposal(serde_json::json!({"id": note_id}), valid_payload());
		let by_note_id = proposal(serde_json::json!({"note_id": note_id}), valid_payload());

		assert_eq!(payload::target_note_id(&by_id).expect("id should parse"), note_id);
		assert_eq!(payload::target_note_id(&by_note_id).expect("note_id should parse"), note_id);
	}

	#[test]
	fn promotion_refs_preserve_schema_and_review_context() {
		let proposal = proposal(serde_json::json!({}), valid_payload());
		let note_id = Uuid::from_u128(99);
		let target_ref = payload::promoted_memory_target_ref(note_id, OffsetDateTime::UNIX_EPOCH);
		let source_ref = payload::promotion_source_ref(
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
}
