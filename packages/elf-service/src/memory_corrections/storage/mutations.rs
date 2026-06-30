use serde_json::Value;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Error, Result,
	memory_corrections::{
		storage::{RestoreNoteArgs, lifecycle, versions},
		types::MemoryCorrectionAction,
		validation::{self},
	},
};
use elf_storage::models::MemoryNote;

pub(in crate::memory_corrections) async fn supersede_note(
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
	note.source_ref = validation::correction_source_ref_for(
		MemoryCorrectionAction::Supersede,
		&prev_snapshot,
		correction_source_ref,
		reason,
		actor_agent_id,
		now,
		None,
	);

	lifecycle::update_note_lifecycle(tx, note).await?;

	let version_id = versions::insert_correction_version(
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

pub(in crate::memory_corrections) async fn delete_note(
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
	note.source_ref = validation::correction_source_ref_for(
		MemoryCorrectionAction::Delete,
		&prev_snapshot,
		correction_source_ref,
		reason,
		actor_agent_id,
		now,
		None,
	);

	lifecycle::update_note_lifecycle(tx, note).await?;

	let version_id = versions::insert_correction_version(
		tx,
		note,
		"DELETE",
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

pub(in crate::memory_corrections) async fn restore_note(
	tx: &mut Transaction<'_, Postgres>,
	note: &mut MemoryNote,
	args: RestoreNoteArgs<'_>,
) -> Result<Option<Uuid>> {
	if note.status == "active" {
		return Ok(None);
	}

	let (restore_version_id, restore_snapshot) =
		versions::load_restore_snapshot(tx, note.note_id, args.restore_version_id).await?;
	let prev_snapshot = crate::note_snapshot(note);

	validation::apply_restore_snapshot(note, &restore_snapshot, args.now)?;

	note.embedding_version = args.embedding_version.to_string();
	note.source_ref = validation::correction_source_ref_for(
		MemoryCorrectionAction::Restore,
		&restore_snapshot,
		args.correction_source_ref,
		args.reason,
		args.actor_agent_id,
		args.now,
		Some(restore_version_id),
	);

	lifecycle::update_note_restored(tx, note).await?;

	let version_id = versions::insert_correction_version(
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
