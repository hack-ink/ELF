use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Result,
	structured_fields::{self, StructuredFields},
};
use elf_storage::models::MemoryNote;

pub(super) async fn update_memory_note_tx(
	tx: &mut Transaction<'_, Postgres>,
	memory_note: &MemoryNote,
) -> Result<()> {
	sqlx::query(
		"\
UPDATE memory_notes
SET
	text = $1,
	importance = $2,
	confidence = $3,
	updated_at = $4,
	expires_at = $5,
	source_ref = $6
WHERE note_id = $7",
	)
	.bind(memory_note.text.as_str())
	.bind(memory_note.importance)
	.bind(memory_note.confidence)
	.bind(memory_note.updated_at)
	.bind(memory_note.expires_at)
	.bind(&memory_note.source_ref)
	.bind(memory_note.note_id)
	.execute(&mut **tx)
	.await?;

	Ok(())
}

pub(super) async fn insert_memory_note_tx(
	tx: &mut Transaction<'_, Postgres>,
	memory_note: &MemoryNote,
) -> Result<()> {
	sqlx::query(
		"\
INSERT INTO memory_notes (
	note_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	type,
	key,
	text,
	importance,
	confidence,
	status,
	created_at,
	updated_at,
	expires_at,
	embedding_version,
	source_ref,
	hit_count,
	last_hit_at
)
VALUES (
	$1,
	$2,
	$3,
	$4,
	$5,
	$6,
	$7,
	$8,
	$9,
	$10,
	$11,
	$12,
	$13,
	$14,
	$15,
	$16,
	$17,
	$18
)",
	)
	.bind(memory_note.note_id)
	.bind(memory_note.tenant_id.as_str())
	.bind(memory_note.project_id.as_str())
	.bind(memory_note.agent_id.as_str())
	.bind(memory_note.scope.as_str())
	.bind(memory_note.r#type.as_str())
	.bind(memory_note.key.as_deref())
	.bind(memory_note.text.as_str())
	.bind(memory_note.importance)
	.bind(memory_note.confidence)
	.bind(memory_note.status.as_str())
	.bind(memory_note.created_at)
	.bind(memory_note.updated_at)
	.bind(memory_note.expires_at)
	.bind(memory_note.embedding_version.as_str())
	.bind(&memory_note.source_ref)
	.bind(memory_note.hit_count)
	.bind(memory_note.last_hit_at)
	.execute(&mut **tx)
	.await?;

	Ok(())
}

pub(super) async fn upsert_structured_fields_tx(
	tx: &mut Transaction<'_, Postgres>,
	structured: Option<&StructuredFields>,
	note_id: Uuid,
	now: OffsetDateTime,
) -> Result<()> {
	if let Some(structured) = structured
		&& !structured.is_effectively_empty()
	{
		structured_fields::upsert_structured_fields_tx(tx, note_id, structured, now).await?;
	}

	Ok(())
}
