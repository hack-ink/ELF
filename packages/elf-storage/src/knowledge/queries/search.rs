use sqlx::PgExecutor;

use crate::{Result, knowledge::types::KnowledgePageSearchRow};

/// Searches derived knowledge page sections by page and section text.
pub async fn search_knowledge_page_sections<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	page_kind: Option<&str>,
	query_pattern: &str,
	limit: i64,
) -> Result<Vec<KnowledgePageSearchRow>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, KnowledgePageSearchRow>(
		"\
WITH page_lint AS (
	SELECT
		page_id,
		count(*) FILTER (WHERE severity = 'error') AS error_count,
		count(*) FILTER (WHERE severity = 'warning') AS warning_count,
		count(*) FILTER (WHERE severity = 'info') AS info_count
	FROM knowledge_page_lint_findings
	GROUP BY page_id
),
section_refs AS (
	SELECT section_id, count(*) AS source_ref_count
	FROM knowledge_page_source_refs
	GROUP BY section_id
)
SELECT
	p.page_id,
	p.page_kind,
	p.page_key,
	p.title,
	p.status,
	p.source_coverage,
	p.rebuild_metadata,
	p.updated_at AS page_updated_at,
	p.rebuilt_at,
	s.section_id,
	s.section_key,
	s.heading,
	s.role,
	s.content,
	s.ordinal,
	s.citations,
	s.unsupported_reason,
	COALESCE(page_lint.error_count, 0)::bigint AS lint_error_count,
	COALESCE(page_lint.warning_count, 0)::bigint AS lint_warning_count,
	COALESCE(page_lint.info_count, 0)::bigint AS lint_info_count,
	COALESCE(section_refs.source_ref_count, 0)::bigint AS section_source_ref_count
FROM knowledge_pages p
JOIN knowledge_page_sections s ON s.page_id = p.page_id
LEFT JOIN page_lint ON page_lint.page_id = p.page_id
LEFT JOIN section_refs ON section_refs.section_id = s.section_id
WHERE p.tenant_id = $1
	AND p.project_id = $2
	AND p.status IN ('active', 'stale')
	AND ($3::text IS NULL OR p.page_kind = $3)
	AND (
		lower(p.title) LIKE $4
		OR lower(p.page_key) LIKE $4
		OR lower(s.heading) LIKE $4
		OR lower(s.content) LIKE $4
	)
ORDER BY
	CASE
		WHEN lower(p.title) LIKE $4 THEN 4
		WHEN lower(s.heading) LIKE $4 THEN 3
		WHEN lower(p.page_key) LIKE $4 THEN 2
		ELSE 1
	END DESC,
	p.updated_at DESC,
	s.ordinal ASC,
	p.page_id DESC
LIMIT $5",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(page_kind)
	.bind(query_pattern)
	.bind(limit)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
