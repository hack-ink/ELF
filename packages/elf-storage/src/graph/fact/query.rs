use crate::graph::{GraphFact, OffsetDateTime, PgConnection, Result, Uuid};

/// Fetches active facts for one subject entity at the provided point in time.
pub async fn fetch_active_facts_for_subject(
	executor: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	scope: &str,
	subject_entity_id: Uuid,
	now: OffsetDateTime,
) -> Result<Vec<GraphFact>> {
	let rows = sqlx::query_as::<_, GraphFact>(
		"\
SELECT
	fact_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	subject_entity_id,
	predicate,
	predicate_id,
	object_entity_id,
	object_value,
	valid_from,
	valid_to,
	created_at,
	updated_at
FROM graph_facts
WHERE tenant_id = $1
	AND project_id = $2
	AND scope = $3
	AND subject_entity_id = $4
	AND valid_from <= $5
	AND (valid_to IS NULL OR valid_to > $5)",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(scope)
	.bind(subject_entity_id)
	.bind(now)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}
