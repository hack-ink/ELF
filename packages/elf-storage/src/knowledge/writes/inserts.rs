use sqlx::PgExecutor;

use crate::{
	Result,
	knowledge::types::{
		KnowledgePageLintFindingInsert, KnowledgePageSectionInsert, KnowledgePageSourceRefInsert,
	},
};

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
