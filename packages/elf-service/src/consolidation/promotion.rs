use serde_json::Value;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use super::types::PromotedMemoryPayload;
use crate::{
	Error, InsertVersionArgs, Result,
	access::{self, ORG_PROJECT_ID},
};
use elf_config::Config;
use elf_domain::{
	ttl,
	writegate::{self, NoteInput},
};
use elf_storage::{
	models::{ConsolidationProposal, MemoryNote},
	queries,
};

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

fn invalid_score(score: f32) -> bool {
	!score.is_finite() || !(0.0..=1.0).contains(&score)
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

fn normalized_optional_string(value: Option<String>) -> Option<String> {
	value.map(|raw| raw.trim().to_string()).filter(|trimmed| !trimmed.is_empty())
}

pub(super) fn promoted_memory_scope(
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

pub(super) fn promoted_memory_project_id<'a>(proposal_project_id: &'a str, scope: &str) -> &'a str {
	if scope == "org_shared" { ORG_PROJECT_ID } else { proposal_project_id }
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

pub(super) fn promoted_memory_target_ref(note_id: Uuid, now: OffsetDateTime) -> Value {
	serde_json::json!({
		"schema": "elf.memory_record_ref/v1",
		"kind": "note",
		"id": note_id,
		"status": "active",
		"applied_at": now,
	})
}

pub(super) async fn create_promoted_memory_note(
	tx: &mut Transaction<'_, Postgres>,
	proposal: &ConsolidationProposal,
	reviewer_agent_id: &str,
	review_comment: Option<&str>,
	cfg: &Config,
	now: OffsetDateTime,
) -> Result<Uuid> {
	let payload = decode_promoted_memory_payload(proposal)?;
	let scope = promoted_memory_scope(&payload, "agent_private")?;

	validate_promoted_memory_payload(&payload, &scope, cfg)?;

	let project_id = promoted_memory_project_id(proposal.project_id.as_str(), &scope);
	let note_type = payload.note_type;
	let expires_at = ttl::compute_expires_at(payload.ttl_days, &note_type, cfg, now);
	let source_ref =
		promotion_source_ref(proposal, &payload.source_ref, reviewer_agent_id, review_comment, now);
	let note_id = Uuid::new_v4();

	access::ensure_active_project_scope_grant(
		&mut **tx,
		proposal.tenant_id.as_str(),
		project_id,
		scope.as_str(),
		proposal.agent_id.as_str(),
	)
	.await?;

	let note = MemoryNote {
		note_id,
		tenant_id: proposal.tenant_id.clone(),
		project_id: project_id.to_string(),
		agent_id: proposal.agent_id.clone(),
		scope,
		r#type: note_type,
		key: normalized_optional_string(payload.key),
		text: payload.text,
		importance: payload.importance.unwrap_or(proposal.confidence),
		confidence: payload.confidence.unwrap_or(proposal.confidence),
		status: "active".to_string(),
		created_at: now,
		updated_at: now,
		expires_at,
		embedding_version: crate::embedding_version(cfg),
		source_ref,
		hit_count: 0,
		last_hit_at: None,
	};

	queries::insert_note(&mut **tx, &note).await?;
	crate::insert_version(
		&mut **tx,
		InsertVersionArgs {
			note_id,
			op: "ADD",
			prev_snapshot: None,
			new_snapshot: Some(crate::note_snapshot(&note)),
			reason: "consolidation_apply.create_derived_note",
			actor: reviewer_agent_id,
			ts: now,
		},
	)
	.await?;
	crate::enqueue_outbox_tx(&mut **tx, note_id, "UPSERT", &note.embedding_version, now).await?;

	Ok(note_id)
}

pub(super) async fn update_promoted_memory_note(
	tx: &mut Transaction<'_, Postgres>,
	proposal: &ConsolidationProposal,
	reviewer_agent_id: &str,
	review_comment: Option<&str>,
	cfg: &Config,
	now: OffsetDateTime,
) -> Result<Uuid> {
	let payload = decode_promoted_memory_payload(proposal)?;
	let note_id = target_note_id(proposal)?;
	let mut note = sqlx::query_as::<_, MemoryNote>(
		"\
SELECT *
FROM memory_notes
WHERE note_id = $1 AND tenant_id = $2 AND project_id IN ($3, $4)
FOR UPDATE",
	)
	.bind(note_id)
	.bind(proposal.tenant_id.as_str())
	.bind(proposal.project_id.as_str())
	.bind(ORG_PROJECT_ID)
	.fetch_optional(&mut **tx)
	.await?
	.ok_or_else(|| Error::InvalidRequest {
		message: "Target memory note was not found.".to_string(),
	})?;

	if note.status != "active" {
		return Err(Error::InvalidRequest {
			message: "Only active target memory can be updated by proposal apply.".to_string(),
		});
	}
	if note.agent_id != proposal.agent_id {
		return Err(Error::InvalidRequest {
			message: "Target memory note owner does not match the proposal owner.".to_string(),
		});
	}

	let scope = promoted_memory_scope(&payload, note.scope.as_str())?;

	validate_promoted_memory_payload(&payload, &scope, cfg)?;

	let project_id = promoted_memory_project_id(proposal.project_id.as_str(), &scope);
	let prev_snapshot = crate::note_snapshot(&note);

	note.project_id = project_id.to_string();
	note.scope = scope;
	note.r#type = payload.note_type;
	note.key = normalized_optional_string(payload.key);
	note.text = payload.text;
	note.importance = payload.importance.unwrap_or(note.importance);
	note.confidence = payload.confidence.unwrap_or(note.confidence);

	if payload.ttl_days.is_some() {
		note.expires_at = ttl::compute_expires_at(payload.ttl_days, &note.r#type, cfg, now);
	}

	note.updated_at = now;
	note.source_ref =
		promotion_source_ref(proposal, &payload.source_ref, reviewer_agent_id, review_comment, now);

	access::ensure_active_project_scope_grant(
		&mut **tx,
		note.tenant_id.as_str(),
		note.project_id.as_str(),
		note.scope.as_str(),
		note.agent_id.as_str(),
	)
	.await?;

	update_promoted_note_row(tx, &note).await?;

	crate::insert_version(
		&mut **tx,
		InsertVersionArgs {
			note_id,
			op: "UPDATE",
			prev_snapshot: Some(prev_snapshot),
			new_snapshot: Some(crate::note_snapshot(&note)),
			reason: "consolidation_apply.update_derived_note",
			actor: reviewer_agent_id,
			ts: now,
		},
	)
	.await?;
	crate::enqueue_outbox_tx(&mut **tx, note_id, "UPSERT", &note.embedding_version, now).await?;

	Ok(note_id)
}

async fn update_promoted_note_row(
	tx: &mut Transaction<'_, Postgres>,
	note: &MemoryNote,
) -> Result<()> {
	sqlx::query(
		"\
UPDATE memory_notes
SET
	project_id = $1,
	scope = $2,
	type = $3,
	key = $4,
	text = $5,
	importance = $6,
	confidence = $7,
	updated_at = $8,
	expires_at = $9,
	source_ref = $10
WHERE note_id = $11",
	)
	.bind(note.project_id.as_str())
	.bind(note.scope.as_str())
	.bind(note.r#type.as_str())
	.bind(note.key.as_deref())
	.bind(note.text.as_str())
	.bind(note.importance)
	.bind(note.confidence)
	.bind(note.updated_at)
	.bind(note.expires_at)
	.bind(&note.source_ref)
	.bind(note.note_id)
	.execute(&mut **tx)
	.await?;

	Ok(())
}
