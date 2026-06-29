use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{
	Result,
	knowledge::types::{KnowledgeDocChunkSource, KnowledgeDocSource},
};

/// Fetches active Source Library documents by identifier for a knowledge page rebuild.
pub async fn fetch_knowledge_doc_sources<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	agent_id: Option<&str>,
	allowed_scopes: &[String],
	doc_ids: &[Uuid],
) -> Result<Vec<KnowledgeDocSource>>
where
	E: PgExecutor<'e>,
{
	if doc_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, KnowledgeDocSource>(
		"\
SELECT
	doc_id,
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
WHERE tenant_id = $1
	AND project_id = $2
	AND ($3::text IS NULL OR scope <> 'agent_private' OR agent_id = $3)
	AND scope = ANY($4::text[])
	AND doc_id = ANY($5::uuid[])
	AND status = 'active'
ORDER BY updated_at ASC, doc_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(agent_id)
	.bind(allowed_scopes)
	.bind(doc_ids)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Fetches active Source Library document chunks by identifier for a knowledge page rebuild.
pub async fn fetch_knowledge_doc_chunk_sources<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	agent_id: Option<&str>,
	allowed_scopes: &[String],
	chunk_ids: &[Uuid],
) -> Result<Vec<KnowledgeDocChunkSource>>
where
	E: PgExecutor<'e>,
{
	if chunk_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, KnowledgeDocChunkSource>(
		"\
SELECT
	c.chunk_id,
	c.doc_id,
	d.agent_id,
	d.scope,
	d.doc_type,
	d.status,
	d.title,
	COALESCE(d.source_ref, '{}'::jsonb) AS source_ref,
	d.content_hash AS doc_content_hash,
	d.updated_at AS doc_updated_at,
	c.chunk_index,
	c.start_offset,
	c.end_offset,
	c.chunk_text,
	c.chunk_hash,
	c.created_at AS chunk_created_at
FROM doc_chunks c
JOIN doc_documents d ON d.doc_id = c.doc_id
WHERE d.tenant_id = $1
	AND d.project_id = $2
	AND ($3::text IS NULL OR d.scope <> 'agent_private' OR d.agent_id = $3)
	AND d.scope = ANY($4::text[])
	AND c.chunk_id = ANY($5::uuid[])
	AND d.status = 'active'
ORDER BY d.updated_at ASC, c.chunk_index ASC, c.chunk_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(agent_id)
	.bind(allowed_scopes)
	.bind(chunk_ids)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
