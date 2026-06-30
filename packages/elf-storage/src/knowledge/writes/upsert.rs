use sqlx::PgExecutor;

use crate::{Result, knowledge::types::KnowledgePageUpsert, models::KnowledgePage};

/// Upserts one derived knowledge page and returns the persisted row.
pub async fn upsert_knowledge_page<'e, E>(
	executor: E,
	args: KnowledgePageUpsert<'_>,
) -> Result<KnowledgePage>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, KnowledgePage>(
		"\
INSERT INTO knowledge_pages (
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
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$14,$14)
ON CONFLICT (tenant_id, project_id, page_kind, page_key) DO UPDATE
SET
	title = EXCLUDED.title,
	contract_schema = EXCLUDED.contract_schema,
	status = EXCLUDED.status,
	rebuild_source_hash = EXCLUDED.rebuild_source_hash,
	content_hash = EXCLUDED.content_hash,
	source_coverage = EXCLUDED.source_coverage,
	source_snapshot = EXCLUDED.source_snapshot,
	rebuild_metadata = EXCLUDED.rebuild_metadata,
	updated_at = EXCLUDED.updated_at,
	rebuilt_at = EXCLUDED.rebuilt_at
RETURNING
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
	rebuilt_at",
	)
	.bind(args.page_id)
	.bind(args.tenant_id)
	.bind(args.project_id)
	.bind(args.page_kind)
	.bind(args.page_key)
	.bind(args.title)
	.bind(args.contract_schema)
	.bind(args.status)
	.bind(args.rebuild_source_hash)
	.bind(args.content_hash)
	.bind(args.source_coverage)
	.bind(args.source_snapshot)
	.bind(args.rebuild_metadata)
	.bind(args.now)
	.fetch_one(executor)
	.await?;

	Ok(row)
}
