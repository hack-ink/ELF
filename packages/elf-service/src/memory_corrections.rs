//! Review-backed memory correction and rollback APIs.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Error, InsertVersionArgs, NoteOp, Result, access::ORG_PROJECT_ID};
use elf_config::Scopes;
use elf_storage::models::MemoryNote;

/// Review-backed correction action for an approved memory record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCorrectionAction {
	/// Mark the memory as superseded while retaining historical readback.
	Supersede,
	/// Tombstone the memory while retaining historical readback.
	Delete,
	/// Restore the latest prior active snapshot from the memory ledger.
	Restore,
}
impl MemoryCorrectionAction {
	/// Returns the canonical action string.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Supersede => "supersede",
			Self::Delete => "delete",
			Self::Restore => "restore",
		}
	}
}

impl ElfService {
	/// Applies a review-backed memory correction and writes an audit version row.
	pub async fn memory_correction_apply(
		&self,
		req: MemoryCorrectionRequest,
	) -> Result<MemoryCorrectionResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let actor_agent_id = req.actor_agent_id.trim();
		let reason = req.reason.trim();

		validate_correction_request(
			tenant_id,
			project_id,
			actor_agent_id,
			reason,
			&req.source_ref,
		)?;

		let now = OffsetDateTime::now_utc();
		let mut tx = self.db.pool.begin().await?;
		let mut note =
			load_note_for_correction(&mut tx, req.note_id, tenant_id, project_id).await?;

		validate_write_scope(&note, &self.cfg.scopes)?;

		let version_id = match req.action {
			MemoryCorrectionAction::Supersede =>
				supersede_note(&mut tx, &mut note, actor_agent_id, reason, &req.source_ref, now)
					.await?,
			MemoryCorrectionAction::Delete =>
				delete_note(&mut tx, &mut note, actor_agent_id, reason, &req.source_ref, now)
					.await?,
			MemoryCorrectionAction::Restore => {
				let embed_version = crate::embedding_version(&self.cfg);

				restore_note(
					&mut tx,
					&mut note,
					RestoreNoteArgs {
						actor_agent_id,
						reason,
						correction_source_ref: &req.source_ref,
						restore_version_id: req.restore_version_id,
						embedding_version: embed_version.as_str(),
						now,
					},
				)
				.await?
			},
		};
		let op = match (req.action, version_id) {
			(_, None) => NoteOp::None,
			(MemoryCorrectionAction::Delete, Some(_)) => NoteOp::Delete,
			(MemoryCorrectionAction::Supersede | MemoryCorrectionAction::Restore, Some(_)) =>
				NoteOp::Update,
		};

		tx.commit().await?;

		Ok(MemoryCorrectionResponse {
			note_id: note.note_id,
			action: req.action,
			op,
			status: note.status,
			version_id,
		})
	}
}

/// Request payload for applying a memory correction.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryCorrectionRequest {
	/// Tenant that owns the memory.
	pub tenant_id: String,
	/// Project that owns the memory.
	pub project_id: String,
	/// Reviewer or policy actor applying the correction.
	pub actor_agent_id: String,
	/// Identifier of the memory note being corrected.
	pub note_id: Uuid,
	/// Correction action to apply.
	pub action: MemoryCorrectionAction,
	/// Reviewer or policy reason for the correction.
	pub reason: String,
	/// Source reference or review record that justifies the correction.
	pub source_ref: Value,
	/// Optional ledger version to restore from. Defaults to the latest supersede/delete snapshot.
	pub restore_version_id: Option<Uuid>,
}

/// Response returned after applying a memory correction.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryCorrectionResponse {
	/// Identifier of the corrected memory note.
	pub note_id: Uuid,
	/// Correction action that was requested.
	pub action: MemoryCorrectionAction,
	/// Storage operation applied to the memory record.
	pub op: NoteOp,
	/// Current lifecycle status after the correction.
	pub status: String,
	/// Version row written for this correction, when a change occurred.
	pub version_id: Option<Uuid>,
}

struct RestoreNoteArgs<'a> {
	actor_agent_id: &'a str,
	reason: &'a str,
	correction_source_ref: &'a Value,
	restore_version_id: Option<Uuid>,
	embedding_version: &'a str,
	now: OffsetDateTime,
}

fn validate_correction_request(
	tenant_id: &str,
	project_id: &str,
	actor_agent_id: &str,
	reason: &str,
	source_ref: &Value,
) -> Result<()> {
	if tenant_id.is_empty() || project_id.is_empty() || actor_agent_id.is_empty() {
		return Err(Error::InvalidRequest {
			message: "tenant_id, project_id, and actor_agent_id are required.".to_string(),
		});
	}
	if reason.is_empty() {
		return Err(Error::InvalidRequest { message: "reason must not be empty.".to_string() });
	}
	if !is_non_empty_object(source_ref) {
		return Err(Error::InvalidRequest {
			message: "source_ref must be a non-empty JSON object.".to_string(),
		});
	}

	Ok(())
}

fn validate_write_scope(note: &MemoryNote, scopes: &Scopes) -> Result<()> {
	if !scopes.allowed.iter().any(|scope| scope == &note.scope) {
		return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
	}

	let write_allowed = match note.scope.as_str() {
		"agent_private" => scopes.write_allowed.agent_private,
		"project_shared" => scopes.write_allowed.project_shared,
		"org_shared" => scopes.write_allowed.org_shared,
		_ => false,
	};

	if write_allowed {
		Ok(())
	} else {
		Err(Error::ScopeDenied { message: "Scope is not writable.".to_string() })
	}
}

fn apply_restore_snapshot(
	note: &mut MemoryNote,
	snapshot: &Value,
	now: OffsetDateTime,
) -> Result<()> {
	let status = required_string(snapshot, "status")?;

	if status != "active" {
		return Err(Error::InvalidRequest {
			message: "Restore snapshot must represent an active memory.".to_string(),
		});
	}

	note.scope = required_string(snapshot, "scope")?;
	note.r#type = required_string(snapshot, "type")?;
	note.key = optional_string(snapshot, "key")?;
	note.text = required_string(snapshot, "text")?;
	note.importance = required_f32(snapshot, "importance")?;
	note.confidence = required_f32(snapshot, "confidence")?;
	note.status = status;
	note.updated_at = now;
	note.expires_at = optional_offset_datetime(snapshot, "expires_at")?;

	Ok(())
}

fn correction_source_ref_for(
	action: MemoryCorrectionAction,
	prior_snapshot: &Value,
	correction_source_ref: &Value,
	reason: &str,
	actor_agent_id: &str,
	now: OffsetDateTime,
	restore_version_id: Option<Uuid>,
) -> Value {
	serde_json::json!({
		"schema": "elf.memory_correction/v1",
		"action": action.as_str(),
		"reason": reason,
		"actor_agent_id": actor_agent_id,
		"ts": now,
		"restore_version_id": restore_version_id,
		"prior_source_ref": prior_snapshot.get("source_ref").cloned().unwrap_or_else(empty_object),
		"prior_snapshot": prior_snapshot,
		"correction_source_ref": correction_source_ref,
	})
}

fn is_non_empty_object(value: &Value) -> bool {
	matches!(value, Value::Object(map) if !map.is_empty())
}

fn required_string(snapshot: &Value, field: &'static str) -> Result<String> {
	snapshot
		.get(field)
		.and_then(Value::as_str)
		.map(str::to_string)
		.filter(|value| !value.trim().is_empty())
		.ok_or_else(|| Error::InvalidRequest {
			message: format!("Restore snapshot field {field} must be a non-empty string."),
		})
}

fn optional_string(snapshot: &Value, field: &'static str) -> Result<Option<String>> {
	match snapshot.get(field) {
		None | Some(Value::Null) => Ok(None),
		Some(Value::String(value)) => Ok(Some(value.clone())),
		_ => Err(Error::InvalidRequest {
			message: format!("Restore snapshot field {field} must be a string or null."),
		}),
	}
}

fn required_f32(snapshot: &Value, field: &'static str) -> Result<f32> {
	let Some(value) = snapshot.get(field).and_then(Value::as_f64) else {
		return Err(Error::InvalidRequest {
			message: format!("Restore snapshot field {field} must be a number."),
		});
	};

	if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
		return Err(Error::InvalidRequest {
			message: format!("Restore snapshot field {field} is out of range."),
		});
	}

	Ok(value as f32)
}

fn optional_offset_datetime(
	snapshot: &Value,
	field: &'static str,
) -> Result<Option<OffsetDateTime>> {
	let Some(value) = snapshot.get(field) else {
		return Ok(None);
	};

	serde_json::from_value(value.clone()).map_err(|err| Error::InvalidRequest {
		message: format!("Restore snapshot field {field} is not a valid timestamp: {err}."),
	})
}

fn empty_object() -> Value {
	Value::Object(Map::new())
}

async fn load_note_for_correction(
	tx: &mut Transaction<'_, Postgres>,
	note_id: Uuid,
	tenant_id: &str,
	project_id: &str,
) -> Result<MemoryNote> {
	sqlx::query_as::<_, MemoryNote>(
		"\
SELECT *
FROM memory_notes
WHERE note_id = $1 AND tenant_id = $2 AND project_id IN ($3, $4)
FOR UPDATE",
	)
	.bind(note_id)
	.bind(tenant_id)
	.bind(project_id)
	.bind(ORG_PROJECT_ID)
	.fetch_optional(&mut **tx)
	.await?
	.ok_or_else(|| Error::InvalidRequest { message: "Note not found.".to_string() })
}

async fn supersede_note(
	tx: &mut Transaction<'_, Postgres>,
	note: &mut MemoryNote,
	actor_agent_id: &str,
	reason: &str,
	correction_source_ref: &Value,
	now: OffsetDateTime,
) -> Result<Option<Uuid>> {
	if note.status == "deprecated" {
		return Ok(None);
	}
	if note.status == "deleted" {
		return Err(Error::InvalidRequest {
			message: "Deleted memory must be restored before it can be superseded.".to_string(),
		});
	}

	let prev_snapshot = crate::note_snapshot(note);

	note.status = "deprecated".to_string();
	note.updated_at = now;
	note.source_ref = correction_source_ref_for(
		MemoryCorrectionAction::Supersede,
		&prev_snapshot,
		correction_source_ref,
		reason,
		actor_agent_id,
		now,
		None,
	);

	update_note_lifecycle(tx, note).await?;

	let version_id = insert_correction_version(
		tx,
		note,
		"DEPRECATE",
		prev_snapshot,
		actor_agent_id,
		reason,
		now,
	)
	.await?;

	crate::enqueue_outbox_tx(&mut **tx, note.note_id, "DELETE", &note.embedding_version, now)
		.await?;

	Ok(Some(version_id))
}

async fn delete_note(
	tx: &mut Transaction<'_, Postgres>,
	note: &mut MemoryNote,
	actor_agent_id: &str,
	reason: &str,
	correction_source_ref: &Value,
	now: OffsetDateTime,
) -> Result<Option<Uuid>> {
	if note.status == "deleted" {
		return Ok(None);
	}

	let prev_snapshot = crate::note_snapshot(note);

	note.status = "deleted".to_string();
	note.updated_at = now;
	note.source_ref = correction_source_ref_for(
		MemoryCorrectionAction::Delete,
		&prev_snapshot,
		correction_source_ref,
		reason,
		actor_agent_id,
		now,
		None,
	);

	update_note_lifecycle(tx, note).await?;

	let version_id =
		insert_correction_version(tx, note, "DELETE", prev_snapshot, actor_agent_id, reason, now)
			.await?;

	crate::enqueue_outbox_tx(&mut **tx, note.note_id, "DELETE", &note.embedding_version, now)
		.await?;

	Ok(Some(version_id))
}

async fn restore_note(
	tx: &mut Transaction<'_, Postgres>,
	note: &mut MemoryNote,
	args: RestoreNoteArgs<'_>,
) -> Result<Option<Uuid>> {
	if note.status == "active" {
		return Ok(None);
	}

	let (restore_version_id, restore_snapshot) =
		load_restore_snapshot(tx, note.note_id, args.restore_version_id).await?;
	let prev_snapshot = crate::note_snapshot(note);

	apply_restore_snapshot(note, &restore_snapshot, args.now)?;

	note.embedding_version = args.embedding_version.to_string();
	note.source_ref = correction_source_ref_for(
		MemoryCorrectionAction::Restore,
		&restore_snapshot,
		args.correction_source_ref,
		args.reason,
		args.actor_agent_id,
		args.now,
		Some(restore_version_id),
	);

	update_note_restored(tx, note).await?;

	let version_id = insert_correction_version(
		tx,
		note,
		"RESTORE",
		prev_snapshot,
		args.actor_agent_id,
		args.reason,
		args.now,
	)
	.await?;

	crate::enqueue_outbox_tx(&mut **tx, note.note_id, "UPSERT", &note.embedding_version, args.now)
		.await?;

	Ok(Some(version_id))
}

async fn update_note_lifecycle(
	tx: &mut Transaction<'_, Postgres>,
	note: &MemoryNote,
) -> Result<()> {
	sqlx::query(
		"\
UPDATE memory_notes
SET status = $1, updated_at = $2, source_ref = $3
WHERE note_id = $4",
	)
	.bind(note.status.as_str())
	.bind(note.updated_at)
	.bind(&note.source_ref)
	.bind(note.note_id)
	.execute(&mut **tx)
	.await?;

	Ok(())
}

async fn update_note_restored(tx: &mut Transaction<'_, Postgres>, note: &MemoryNote) -> Result<()> {
	sqlx::query(
		"\
UPDATE memory_notes
SET
	scope = $1,
	type = $2,
	key = $3,
	text = $4,
	importance = $5,
	confidence = $6,
	status = $7,
	updated_at = $8,
	expires_at = $9,
	embedding_version = $10,
	source_ref = $11
WHERE note_id = $12",
	)
	.bind(note.scope.as_str())
	.bind(note.r#type.as_str())
	.bind(note.key.as_deref())
	.bind(note.text.as_str())
	.bind(note.importance)
	.bind(note.confidence)
	.bind(note.status.as_str())
	.bind(note.updated_at)
	.bind(note.expires_at)
	.bind(note.embedding_version.as_str())
	.bind(&note.source_ref)
	.bind(note.note_id)
	.execute(&mut **tx)
	.await?;

	Ok(())
}

async fn insert_correction_version(
	tx: &mut Transaction<'_, Postgres>,
	note: &MemoryNote,
	op: &str,
	prev_snapshot: Value,
	actor_agent_id: &str,
	reason: &str,
	now: OffsetDateTime,
) -> Result<Uuid> {
	let reason = format!("memory_correction.{}: {reason}", op.to_ascii_lowercase());

	crate::insert_version(
		&mut **tx,
		InsertVersionArgs {
			note_id: note.note_id,
			op,
			prev_snapshot: Some(prev_snapshot),
			new_snapshot: Some(crate::note_snapshot(note)),
			reason: reason.as_str(),
			actor: actor_agent_id,
			ts: now,
		},
	)
	.await
}

async fn load_restore_snapshot(
	tx: &mut Transaction<'_, Postgres>,
	note_id: Uuid,
	restore_version_id: Option<Uuid>,
) -> Result<(Uuid, Value)> {
	let row: Option<(Uuid, Value)> = if let Some(version_id) = restore_version_id {
		sqlx::query_as(
			"\
SELECT version_id, prev_snapshot
FROM memory_note_versions
WHERE note_id = $1 AND version_id = $2 AND prev_snapshot IS NOT NULL
LIMIT 1",
		)
		.bind(note_id)
		.bind(version_id)
		.fetch_optional(&mut **tx)
		.await?
	} else {
		sqlx::query_as(
			"\
SELECT version_id, prev_snapshot
FROM memory_note_versions
WHERE note_id = $1
	AND op IN ('DELETE', 'DEPRECATE')
	AND prev_snapshot IS NOT NULL
	AND prev_snapshot ->> 'status' = 'active'
ORDER BY ts DESC, version_id DESC
LIMIT 1",
		)
		.bind(note_id)
		.fetch_optional(&mut **tx)
		.await?
	};

	row.ok_or_else(|| Error::InvalidRequest {
		message: "No restorable memory snapshot was found.".to_string(),
	})
}

#[cfg(test)]
mod tests {
	use time::OffsetDateTime;
	use uuid::Uuid;

	use crate::memory_corrections::{self, MemoryCorrectionAction};
	use elf_storage::models::MemoryNote;

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
			memory_corrections::validate_correction_request(
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
			memory_corrections::validate_correction_request(
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
			memory_corrections::validate_correction_request(
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

		memory_corrections::apply_restore_snapshot(
			&mut note,
			&snapshot,
			OffsetDateTime::UNIX_EPOCH,
		)
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
		let correction = memory_corrections::correction_source_ref_for(
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
}
