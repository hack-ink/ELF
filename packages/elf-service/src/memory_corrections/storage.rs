use serde_json::Value;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, InsertVersionArgs, Result, access::ORG_PROJECT_ID};
use elf_storage::models::MemoryNote;

use super::{
	types::MemoryCorrectionAction,
	validation::{apply_restore_snapshot, correction_source_ref_for},
};

pub(super) struct RestoreNoteArgs<'a> {
	pub(super) actor_agent_id: &'a str,
	pub(super) reason: &'a str,
	pub(super) correction_source_ref: &'a Value,
	pub(super) restore_version_id: Option<Uuid>,
	pub(super) embedding_version: &'a str,
	pub(super) now: OffsetDateTime,
}

pub(super) async fn load_note_for_correction(
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

pub(super) async fn supersede_note(
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

pub(super) async fn delete_note(
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

pub(super) async fn restore_note(
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
