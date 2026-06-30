use sqlx::{Postgres, Transaction};

use crate::Result;
use elf_storage::models::MemoryNote;

pub(super) async fn update_note_lifecycle(
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

pub(super) async fn update_note_restored(
	tx: &mut Transaction<'_, Postgres>,
	note: &MemoryNote,
) -> Result<()> {
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
