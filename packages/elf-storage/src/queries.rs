use color_eyre::Result;
use uuid::Uuid;

use crate::{db::Db, models::MemoryNote};

pub async fn insert_note(db: &Db, note: &MemoryNote) -> Result<()> {
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
	.bind(note.note_id)
	.bind(&note.tenant_id)
	.bind(&note.project_id)
	.bind(&note.agent_id)
	.bind(&note.scope)
	.bind(&note.r#type)
	.bind(&note.key)
	.bind(&note.text)
	.bind(note.importance)
	.bind(note.confidence)
	.bind(&note.status)
	.bind(note.created_at)
	.bind(note.updated_at)
	.bind(note.expires_at)
	.bind(&note.embedding_version)
	.bind(&note.source_ref)
	.bind(note.hit_count)
	.bind(note.last_hit_at)
	.execute(&db.pool)
	.await?;

	Ok(())
}

pub async fn update_note(db: &Db, note: &MemoryNote) -> Result<()> {
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
	.bind(&note.text)
	.bind(note.importance)
	.bind(note.confidence)
	.bind(note.updated_at)
	.bind(note.expires_at)
	.bind(&note.source_ref)
	.bind(note.note_id)
	.execute(&db.pool)
	.await?;

	Ok(())
}

pub async fn delete_note_chunks(db: &Db, note_id: Uuid) -> Result<()> {
	sqlx::query("DELETE FROM memory_note_chunks WHERE note_id = $1")
		.bind(note_id)
		.execute(&db.pool)
		.await?;

	Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_note_chunk(
	db: &Db,
	chunk_id: Uuid,
	note_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: &str,
	embedding_version: &str,
) -> Result<()> {
	sqlx::query(
		"\
INSERT INTO memory_note_chunks (
	chunk_id,
	note_id,
	chunk_index,
	start_offset,
	end_offset,
	text,
	embedding_version
)
VALUES ($1, $2, $3, $4, $5, $6, $7)
ON CONFLICT (chunk_id) DO UPDATE
SET
	text = EXCLUDED.text,
	start_offset = EXCLUDED.start_offset,
	end_offset = EXCLUDED.end_offset",
	)
	.bind(chunk_id)
	.bind(note_id)
	.bind(chunk_index)
	.bind(start_offset)
	.bind(end_offset)
	.bind(text)
	.bind(embedding_version)
	.execute(&db.pool)
	.await?;

	Ok(())
}

pub async fn insert_note_chunk_embedding(
	db: &Db,
	chunk_id: Uuid,
	embedding_version: &str,
	embedding_dim: i32,
	vec: &str,
) -> Result<()> {
	sqlx::query(
		"\
INSERT INTO note_chunk_embeddings (chunk_id, embedding_version, embedding_dim, vec)
VALUES ($1, $2, $3, $4::vector)
ON CONFLICT (chunk_id, embedding_version) DO UPDATE
SET
	embedding_dim = EXCLUDED.embedding_dim,
	vec = EXCLUDED.vec,
	created_at = now()",
	)
	.bind(chunk_id)
	.bind(embedding_version)
	.bind(embedding_dim)
	.bind(vec)
	.execute(&db.pool)
	.await?;

	Ok(())
}
