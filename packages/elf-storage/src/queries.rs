use color_eyre::Result;
use sqlx::{Executor, Postgres, Transaction};
use uuid::Uuid;

use crate::{db::Db, models::MemoryNote};

pub async fn insert_note(db: &Db, note: &MemoryNote) -> Result<()> {
	sqlx::query!(
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
		note.note_id,
		note.tenant_id.as_str(),
		note.project_id.as_str(),
		note.agent_id.as_str(),
		note.scope.as_str(),
		note.r#type.as_str(),
		note.key.as_deref(),
		note.text.as_str(),
		note.importance,
		note.confidence,
		note.status.as_str(),
		note.created_at,
		note.updated_at,
		note.expires_at,
		note.embedding_version.as_str(),
		&note.source_ref,
		note.hit_count,
		note.last_hit_at,
	)
	.execute(&db.pool)
	.await?;

	Ok(())
}

pub async fn update_note(db: &Db, note: &MemoryNote) -> Result<()> {
	sqlx::query!(
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
		note.text.as_str(),
		note.importance,
		note.confidence,
		note.updated_at,
		note.expires_at,
		&note.source_ref,
		note.note_id,
	)
	.execute(&db.pool)
	.await?;

	Ok(())
}

pub async fn delete_note_chunks(db: &Db, note_id: Uuid) -> Result<()> {
	delete_note_chunks_exec(&db.pool, note_id).await?;

	Ok(())
}

pub async fn delete_note_chunks_tx(
	tx: &mut Transaction<'_, Postgres>,
	note_id: Uuid,
) -> Result<()> {
	delete_note_chunks_exec(&mut **tx, note_id).await?;

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
	insert_note_chunk_exec(
		&db.pool,
		chunk_id,
		note_id,
		chunk_index,
		start_offset,
		end_offset,
		text,
		embedding_version,
	)
	.await?;

	Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_note_chunk_tx(
	tx: &mut Transaction<'_, Postgres>,
	chunk_id: Uuid,
	note_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: &str,
	embedding_version: &str,
) -> Result<()> {
	insert_note_chunk_exec(
		&mut **tx,
		chunk_id,
		note_id,
		chunk_index,
		start_offset,
		end_offset,
		text,
		embedding_version,
	)
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
	insert_note_chunk_embedding_exec(&db.pool, chunk_id, embedding_version, embedding_dim, vec)
		.await?;

	Ok(())
}

pub async fn insert_note_chunk_embedding_tx(
	tx: &mut Transaction<'_, Postgres>,
	chunk_id: Uuid,
	embedding_version: &str,
	embedding_dim: i32,
	vec: &str,
) -> Result<()> {
	insert_note_chunk_embedding_exec(&mut **tx, chunk_id, embedding_version, embedding_dim, vec)
		.await?;

	Ok(())
}

async fn delete_note_chunks_exec<'e, E>(executor: E, note_id: Uuid) -> Result<()>
where
	E: Executor<'e, Database = Postgres>,
{
	sqlx::query!("DELETE FROM memory_note_chunks WHERE note_id = $1", note_id)
		.execute(executor)
		.await?;

	Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_note_chunk_exec<'e, E>(
	executor: E,
	chunk_id: Uuid,
	note_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: &str,
	embedding_version: &str,
) -> Result<()>
where
	E: Executor<'e, Database = Postgres>,
{
	sqlx::query!(
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
		chunk_id,
		note_id,
		chunk_index,
		start_offset,
		end_offset,
		text,
		embedding_version,
	)
	.execute(executor)
	.await?;

	Ok(())
}

async fn insert_note_chunk_embedding_exec<'e, E>(
	executor: E,
	chunk_id: Uuid,
	embedding_version: &str,
	embedding_dim: i32,
	vec: &str,
) -> Result<()>
where
	E: Executor<'e, Database = Postgres>,
{
	sqlx::query!(
		"\
	INSERT INTO note_chunk_embeddings (chunk_id, embedding_version, embedding_dim, vec)
	VALUES ($1, $2, $3, $4::text::vector)
	ON CONFLICT (chunk_id, embedding_version) DO UPDATE
	SET
		embedding_dim = EXCLUDED.embedding_dim,
		vec = EXCLUDED.vec,
	created_at = now()",
		chunk_id,
		embedding_version,
		embedding_dim,
		vec,
	)
	.execute(executor)
	.await?;

	Ok(())
}
