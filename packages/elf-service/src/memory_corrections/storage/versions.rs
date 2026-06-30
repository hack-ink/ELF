use serde_json::Value;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, InsertVersionArgs, Result};
use elf_storage::models::MemoryNote;

pub(super) async fn insert_correction_version(
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

pub(super) async fn load_restore_snapshot(
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
