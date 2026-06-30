use sqlx::PgExecutor;
use uuid::Uuid;

use crate::Result;

/// Upserts one dense or sparse embedding vector for a document chunk.
pub async fn insert_doc_chunk_embedding<'e, E>(
	executor: E,
	chunk_id: Uuid,
	embedding_version: &str,
	embedding_dim: i32,
	vec: &str,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO doc_chunk_embeddings (chunk_id, embedding_version, embedding_dim, vec)
VALUES ($1, $2, $3, $4::text::vector)
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
	.execute(executor)
	.await?;

	Ok(())
}
