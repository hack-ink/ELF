use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{Result, knowledge::types::KnowledgeNoteSource};

/// Fetches note sources by identifier for a knowledge page rebuild.
pub async fn fetch_knowledge_note_sources<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	agent_id: Option<&str>,
	allowed_scopes: &[String],
	note_ids: &[Uuid],
) -> Result<Vec<KnowledgeNoteSource>>
where
	E: PgExecutor<'e>,
{
	if note_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, KnowledgeNoteSource>(
		"\
SELECT
	note_id,
	agent_id,
	scope,
	type AS note_type,
	key,
	text,
	importance,
	confidence,
	status,
	created_at,
	updated_at,
	expires_at,
	embedding_version,
	source_ref
FROM memory_notes
WHERE tenant_id = $1
	AND project_id = $2
	AND ($3::text IS NULL OR scope <> 'agent_private' OR agent_id = $3)
	AND scope = ANY($4::text[])
	AND note_id = ANY($5::uuid[])
	AND status = 'active'
	AND (expires_at IS NULL OR expires_at > now())
ORDER BY updated_at ASC, note_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(agent_id)
	.bind(allowed_scopes)
	.bind(note_ids)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
