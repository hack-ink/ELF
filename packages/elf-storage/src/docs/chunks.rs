use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{Result, models::DocChunk};

/// Inserts one document chunk row.
pub async fn insert_doc_chunk<'e, E>(executor: E, chunk: &DocChunk) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO doc_chunks (
	chunk_id,
	doc_id,
	chunk_index,
	start_offset,
	end_offset,
	chunk_text,
	chunk_hash,
	created_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
ON CONFLICT (chunk_id) DO UPDATE
SET
	doc_id = EXCLUDED.doc_id,
	chunk_index = EXCLUDED.chunk_index,
	start_offset = EXCLUDED.start_offset,
	end_offset = EXCLUDED.end_offset,
	chunk_text = EXCLUDED.chunk_text,
	chunk_hash = EXCLUDED.chunk_hash",
	)
	.bind(chunk.chunk_id)
	.bind(chunk.doc_id)
	.bind(chunk.chunk_index)
	.bind(chunk.start_offset)
	.bind(chunk.end_offset)
	.bind(chunk.chunk_text.as_str())
	.bind(chunk.chunk_hash.as_str())
	.bind(chunk.created_at)
	.execute(executor)
	.await?;

	Ok(())
}

/// Lists all chunks for one document in chunk order.
pub async fn list_doc_chunks<'e, E>(executor: E, doc_id: Uuid) -> Result<Vec<DocChunk>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, DocChunk>(
		"\
SELECT
	chunk_id,
	doc_id,
	chunk_index,
	start_offset,
	end_offset,
	chunk_text,
	chunk_hash,
	created_at
FROM doc_chunks
WHERE doc_id = $1
ORDER BY chunk_index ASC",
	)
	.bind(doc_id)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Fetches one document chunk by chunk identifier.
pub async fn get_doc_chunk<'e, E>(executor: E, chunk_id: Uuid) -> Result<Option<DocChunk>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, DocChunk>(
		"\
SELECT
	chunk_id,
	doc_id,
	chunk_index,
	start_offset,
	end_offset,
	chunk_text,
	chunk_hash,
	created_at
FROM doc_chunks
WHERE chunk_id = $1
LIMIT 1",
	)
	.bind(chunk_id)
	.fetch_optional(executor)
	.await?;

	Ok(row)
}
