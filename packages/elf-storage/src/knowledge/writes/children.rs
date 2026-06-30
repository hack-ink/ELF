use sqlx::PgExecutor;
use uuid::Uuid;

use crate::Result;

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
