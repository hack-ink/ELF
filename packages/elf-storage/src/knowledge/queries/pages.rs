use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{Result, models::KnowledgePage};

/// Fetches one knowledge page by identifier.
pub async fn get_knowledge_page<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	page_id: Uuid,
) -> Result<Option<KnowledgePage>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, KnowledgePage>(
		"\
SELECT
	page_id,
	tenant_id,
	project_id,
	page_kind,
	page_key,
	title,
	contract_schema,
	status,
	rebuild_source_hash,
	content_hash,
	source_coverage,
	source_snapshot,
	rebuild_metadata,
	created_at,
	updated_at,
	rebuilt_at
FROM knowledge_pages
WHERE tenant_id = $1 AND project_id = $2 AND page_id = $3
LIMIT 1",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(page_id)
	.fetch_optional(executor)
	.await?;

	Ok(row)
}

/// Fetches one knowledge page by stable page key.
pub async fn get_knowledge_page_by_key<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	page_kind: &str,
	page_key: &str,
) -> Result<Option<KnowledgePage>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, KnowledgePage>(
		"\
SELECT
	page_id,
	tenant_id,
	project_id,
	page_kind,
	page_key,
	title,
	contract_schema,
	status,
	rebuild_source_hash,
	content_hash,
	source_coverage,
	source_snapshot,
	rebuild_metadata,
	created_at,
	updated_at,
	rebuilt_at
FROM knowledge_pages
WHERE tenant_id = $1
	AND project_id = $2
	AND page_kind = $3
	AND page_key = $4
LIMIT 1",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(page_kind)
	.bind(page_key)
	.fetch_optional(executor)
	.await?;

	Ok(row)
}

/// Lists knowledge pages for a tenant and project.
pub async fn list_knowledge_pages<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	page_kind: Option<&str>,
	limit: i64,
) -> Result<Vec<KnowledgePage>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, KnowledgePage>(
		"\
SELECT
	page_id,
	tenant_id,
	project_id,
	page_kind,
	page_key,
	title,
	contract_schema,
	status,
	rebuild_source_hash,
	content_hash,
	source_coverage,
	source_snapshot,
	rebuild_metadata,
	created_at,
	updated_at,
	rebuilt_at
FROM knowledge_pages
WHERE tenant_id = $1
	AND project_id = $2
	AND ($3::text IS NULL OR page_kind = $3)
ORDER BY updated_at DESC, page_id DESC
LIMIT $4",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(page_kind)
	.bind(limit)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Lists knowledge pages that cite at least one changed source.
pub async fn list_knowledge_pages_for_sources<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	page_kind: Option<&str>,
	source_kinds: &[String],
	source_ids: &[Uuid],
	limit: i64,
) -> Result<Vec<KnowledgePage>>
where
	E: PgExecutor<'e>,
{
	if source_kinds.is_empty() || source_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, KnowledgePage>(
		"\
SELECT DISTINCT
	p.page_id,
	p.tenant_id,
	p.project_id,
	p.page_kind,
	p.page_key,
	p.title,
	p.contract_schema,
	p.status,
	p.rebuild_source_hash,
	p.content_hash,
	p.source_coverage,
	p.source_snapshot,
	p.rebuild_metadata,
	p.created_at,
	p.updated_at,
	p.rebuilt_at
FROM knowledge_pages p
JOIN knowledge_page_source_refs r ON r.page_id = p.page_id
JOIN unnest($4::text[], $5::uuid[]) AS changed(source_kind, source_id)
	ON changed.source_kind = r.source_kind
	AND changed.source_id = r.source_id
WHERE p.tenant_id = $1
	AND p.project_id = $2
	AND ($3::text IS NULL OR p.page_kind = $3)
	AND p.status IN ('active', 'stale')
ORDER BY p.updated_at DESC, p.page_id DESC
LIMIT $6",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(page_kind)
	.bind(source_kinds)
	.bind(source_ids)
	.bind(limit)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
