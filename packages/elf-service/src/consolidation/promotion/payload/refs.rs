use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Result};
use elf_storage::models::ConsolidationProposal;

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

pub(in crate::consolidation) fn target_note_id(proposal: &ConsolidationProposal) -> Result<Uuid> {
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

pub(in crate::consolidation) fn promotion_source_ref(
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
