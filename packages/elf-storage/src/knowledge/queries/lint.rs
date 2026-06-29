use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{Result, models::KnowledgePageLintFinding};

/// Lists lint findings for one knowledge page.
pub async fn list_knowledge_page_lint_findings<'e, E>(
	executor: E,
	page_id: Uuid,
) -> Result<Vec<KnowledgePageLintFinding>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, KnowledgePageLintFinding>(
		"\
SELECT
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
FROM knowledge_page_lint_findings
WHERE page_id = $1
ORDER BY severity DESC, created_at ASC, finding_id ASC",
	)
	.bind(page_id)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
