use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{Result, models::KnowledgePageSection};

/// Lists sections for one knowledge page.
pub async fn list_knowledge_page_sections<'e, E>(
	executor: E,
	page_id: Uuid,
) -> Result<Vec<KnowledgePageSection>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, KnowledgePageSection>(
		"\
SELECT
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
FROM knowledge_page_sections
WHERE page_id = $1
ORDER BY ordinal ASC, section_key ASC",
	)
	.bind(page_id)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
