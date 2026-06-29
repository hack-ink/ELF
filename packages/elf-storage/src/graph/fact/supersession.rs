use crate::graph::{OffsetDateTime, PgConnection, Result, Uuid};

#[allow(clippy::too_many_arguments)]
/// Supersedes active facts that conflict with the replacement fact and records supersession rows.
pub async fn supersede_conflicting_active_facts(
	executor: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	scope: &str,
	subject_entity_id: Uuid,
	predicate_id: Uuid,
	to_fact_id: Uuid,
	note_id: Uuid,
	effective_at: OffsetDateTime,
) -> Result<Vec<Uuid>> {
	let superseded: Vec<(Uuid,)> = sqlx::query_as(
		"\
UPDATE graph_facts
SET valid_to = $1, updated_at = now()
WHERE tenant_id = $2
	AND project_id = $3
	AND scope = $4
	AND subject_entity_id = $5
	AND predicate_id = $6
	AND valid_to IS NULL
	AND valid_from <= $1
	AND fact_id <> $7
RETURNING fact_id",
	)
	.bind(effective_at)
	.bind(tenant_id)
	.bind(project_id)
	.bind(scope)
	.bind(subject_entity_id)
	.bind(predicate_id)
	.bind(to_fact_id)
	.fetch_all(&mut *executor)
	.await?;

	for (from_fact_id,) in &superseded {
		sqlx::query(
			"\
INSERT INTO graph_fact_supersessions (
	supersession_id,
	tenant_id,
	project_id,
	from_fact_id,
	to_fact_id,
	note_id,
	effective_at,
	created_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, now())
ON CONFLICT (from_fact_id, to_fact_id, note_id) DO NOTHING",
		)
		.bind(Uuid::new_v4())
		.bind(tenant_id)
		.bind(project_id)
		.bind(*from_fact_id)
		.bind(to_fact_id)
		.bind(note_id)
		.bind(effective_at)
		.execute(&mut *executor)
		.await?;
	}

	Ok(superseded.into_iter().map(|(fact_id,)| fact_id).collect())
}
