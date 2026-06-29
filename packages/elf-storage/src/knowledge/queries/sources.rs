use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{Result, models::KnowledgePageSourceRef};

/// Lists normalized source refs for one knowledge page.
pub async fn list_knowledge_page_source_refs<'e, E>(
	executor: E,
	page_id: Uuid,
) -> Result<Vec<KnowledgePageSourceRef>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, KnowledgePageSourceRef>(
		"\
SELECT
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
FROM knowledge_page_source_refs
WHERE page_id = $1
ORDER BY source_kind ASC, source_id ASC, ref_id ASC",
	)
	.bind(page_id)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Lists normalized source refs for a set of knowledge pages.
pub async fn list_knowledge_page_source_refs_for_pages<'e, E>(
	executor: E,
	page_ids: &[Uuid],
) -> Result<Vec<KnowledgePageSourceRef>>
where
	E: PgExecutor<'e>,
{
	if page_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, KnowledgePageSourceRef>(
		"\
SELECT
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
FROM knowledge_page_source_refs
WHERE page_id = ANY($1::uuid[])
ORDER BY page_id ASC, source_kind ASC, source_id ASC, ref_id ASC",
	)
	.bind(page_ids)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
