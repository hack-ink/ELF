use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{Result, knowledge::types::KnowledgeEventSource};

/// Fetches durable add_event audit sources by decision identifier.
pub async fn fetch_knowledge_event_sources<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	agent_id: Option<&str>,
	allowed_scopes: &[String],
	decision_ids: &[Uuid],
) -> Result<Vec<KnowledgeEventSource>>
where
	E: PgExecutor<'e>,
{
	if decision_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, KnowledgeEventSource>(
		"\
SELECT
	memory_ingest_decisions.decision_id,
	memory_ingest_decisions.agent_id,
	memory_ingest_decisions.scope,
	memory_ingest_decisions.pipeline,
	memory_ingest_decisions.note_type,
	memory_ingest_decisions.note_key,
	memory_ingest_decisions.note_id,
	memory_ingest_decisions.policy_decision,
	memory_ingest_decisions.note_op,
	memory_ingest_decisions.reason_code,
	memory_ingest_decisions.details,
	memory_ingest_decisions.ts
FROM memory_ingest_decisions
JOIN memory_notes note ON note.note_id = memory_ingest_decisions.note_id
WHERE memory_ingest_decisions.tenant_id = $1
	AND memory_ingest_decisions.project_id = $2
	AND ($3::text IS NULL OR memory_ingest_decisions.scope <> 'agent_private' OR memory_ingest_decisions.agent_id = $3)
	AND memory_ingest_decisions.scope = ANY($4::text[])
	AND memory_ingest_decisions.decision_id = ANY($5::uuid[])
	AND memory_ingest_decisions.pipeline = 'add_event'
	AND memory_ingest_decisions.policy_decision IN ('remember', 'update')
	AND note.tenant_id = memory_ingest_decisions.tenant_id
	AND note.project_id = memory_ingest_decisions.project_id
	AND note.status = 'active'
	AND (note.expires_at IS NULL OR note.expires_at > now())
	AND ($3::text IS NULL OR note.scope <> 'agent_private' OR note.agent_id = $3)
	AND note.scope = ANY($4::text[])
ORDER BY memory_ingest_decisions.ts ASC, memory_ingest_decisions.decision_id ASC",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(agent_id)
	.bind(allowed_scopes)
	.bind(decision_ids)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
