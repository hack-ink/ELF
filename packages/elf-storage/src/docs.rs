//! Document persistence queries.

use serde_json::Value;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Result,
	models::{DocChunk, DocDocument},
};

/// Normalizes absent document source metadata to an empty JSON object.
pub fn normalize_source_ref(source_ref: Option<Value>) -> Value {
	source_ref.unwrap_or(Value::Object(Default::default()))
}

/// Inserts one document record into storage.
pub async fn insert_doc_document<'e, E>(executor: E, doc: &DocDocument) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
	INSERT INTO doc_documents (
		doc_id,
		tenant_id,
		project_id,
		agent_id,
		scope,
		doc_type,
		status,
		title,
		source_ref,
		content,
		content_bytes,
		content_hash,
		created_at,
		updated_at
	)
	VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
	)
	.bind(doc.doc_id)
	.bind(doc.tenant_id.as_str())
	.bind(doc.project_id.as_str())
	.bind(doc.agent_id.as_str())
	.bind(doc.scope.as_str())
	.bind(doc.doc_type.as_str())
	.bind(doc.status.as_str())
	.bind(doc.title.as_deref())
	.bind(&doc.source_ref)
	.bind(doc.content.as_str())
	.bind(doc.content_bytes)
	.bind(doc.content_hash.as_str())
	.bind(doc.created_at)
	.bind(doc.updated_at)
	.execute(executor)
	.await?;

	Ok(())
}

/// Fetches one document record by tenant and document identifier.
pub async fn get_doc_document<'e, E>(
	executor: E,
	tenant_id: &str,
	doc_id: Uuid,
) -> Result<Option<DocDocument>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, DocDocument>(
		"\
	SELECT
		doc_id,
		tenant_id,
		project_id,
		agent_id,
		scope,
		doc_type,
		status,
		title,
		COALESCE(source_ref, '{}'::jsonb) AS source_ref,
		content,
		content_bytes,
		content_hash,
		created_at,
		updated_at
FROM doc_documents
WHERE tenant_id = $1 AND doc_id = $2
LIMIT 1",
	)
	.bind(tenant_id)
	.bind(doc_id)
	.fetch_optional(executor)
	.await?;

	Ok(row)
}

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
VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
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

/// Marks one document record as deleted.
pub async fn mark_doc_deleted<'e, E>(
	executor: E,
	tenant_id: &str,
	doc_id: Uuid,
	now: OffsetDateTime,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
UPDATE doc_documents
SET status = 'deleted', updated_at = $1
WHERE tenant_id = $2 AND doc_id = $3",
	)
	.bind(now)
	.bind(tenant_id)
	.bind(doc_id)
	.execute(executor)
	.await?;

	Ok(())
}
