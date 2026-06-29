use sqlx::PgExecutor;
use uuid::Uuid;

use super::types::{
	KnowledgePageLintFindingInsert, KnowledgePageSectionInsert, KnowledgePageSourceRefInsert,
	KnowledgePageUpsert,
};
use crate::{Result, models::KnowledgePage};

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

/// Deletes all section, citation, and lint child rows for a page before rebuild.
pub async fn delete_knowledge_page_children<'e, E>(executor: E, page_id: Uuid) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
	WITH deleted_lint AS (
		DELETE FROM knowledge_page_lint_findings
		WHERE page_id = $1
	),
	deleted_source_refs AS (
		DELETE FROM knowledge_page_source_refs
		WHERE page_id = $1
	)
	DELETE FROM knowledge_page_sections
	WHERE page_id = $1",
	)
	.bind(page_id)
	.execute(executor)
	.await?;

	Ok(())
}

/// Inserts one derived knowledge page section.
pub async fn insert_knowledge_page_section<'e, E>(
	executor: E,
	args: KnowledgePageSectionInsert<'_>,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO knowledge_page_sections (
	section_id,
	page_id,
	section_key,
	heading,
	role,
	content,
	ordinal,
	citations,
	unsupported_reason,
	content_hash,
	created_at,
	updated_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$11)",
	)
	.bind(args.section_id)
	.bind(args.page_id)
	.bind(args.section_key)
	.bind(args.heading)
	.bind(args.role)
	.bind(args.content)
	.bind(args.ordinal)
	.bind(args.citations)
	.bind(args.unsupported_reason)
	.bind(args.content_hash)
	.bind(args.now)
	.execute(executor)
	.await?;

	Ok(())
}

/// Inserts one normalized knowledge page citation/source reference.
pub async fn insert_knowledge_page_source_ref<'e, E>(
	executor: E,
	args: KnowledgePageSourceRefInsert<'_>,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO knowledge_page_source_refs (
	ref_id,
	page_id,
	section_id,
	source_kind,
	source_id,
	source_status,
	source_updated_at,
	source_content_hash,
	source_snapshot,
	citation_metadata,
	created_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
	)
	.bind(args.ref_id)
	.bind(args.page_id)
	.bind(args.section_id)
	.bind(args.source_kind)
	.bind(args.source_id)
	.bind(args.source_status)
	.bind(args.source_updated_at)
	.bind(args.source_content_hash)
	.bind(args.source_snapshot)
	.bind(args.citation_metadata)
	.bind(args.now)
	.execute(executor)
	.await?;

	Ok(())
}

/// Inserts one knowledge page lint finding.
pub async fn insert_knowledge_page_lint_finding<'e, E>(
	executor: E,
	args: KnowledgePageLintFindingInsert<'_>,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO knowledge_page_lint_findings (
	finding_id,
	page_id,
	section_id,
	finding_type,
	severity,
	source_kind,
	source_id,
	message,
	details,
	created_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
	)
	.bind(args.finding_id)
	.bind(args.page_id)
	.bind(args.section_id)
	.bind(args.finding_type)
	.bind(args.severity)
	.bind(args.source_kind)
	.bind(args.source_id)
	.bind(args.message)
	.bind(args.details)
	.bind(args.now)
	.execute(executor)
	.await?;

	Ok(())
}

/// Deletes persisted lint findings for one page.
pub async fn delete_knowledge_page_lint_findings<'e, E>(executor: E, page_id: Uuid) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query("DELETE FROM knowledge_page_lint_findings WHERE page_id = $1")
		.bind(page_id)
		.execute(executor)
		.await?;

	Ok(())
}
