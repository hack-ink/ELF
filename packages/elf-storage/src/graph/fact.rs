use crate::graph::{Error, GraphFact, OffsetDateTime, PgConnection, Result, Uuid};

#[allow(clippy::too_many_arguments)]
/// Inserts a new graph fact row and attaches its evidence note identifiers.
pub async fn insert_fact_with_evidence(
	executor: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	scope: &str,
	subject_entity_id: Uuid,
	predicate: &str,
	predicate_id: Uuid,
	object_entity_id: Option<Uuid>,
	object_value: Option<&str>,
	valid_from: OffsetDateTime,
	valid_to: Option<OffsetDateTime>,
	evidence_note_ids: &[Uuid],
) -> Result<Uuid> {
	if evidence_note_ids.is_empty() {
		return Err(Error::InvalidArgument(
			"graph fact evidence is required; evidence_note_ids must not be empty".to_string(),
		));
	}

	match (object_entity_id, object_value) {
		(Some(_), None) | (None, Some(_)) => (),
		_ => {
			return Err(Error::InvalidArgument(
				"graph fact must provide exactly one of object_entity_id and object_value"
					.to_string(),
			));
		},
	}

	let row: (Uuid,) = sqlx::query_as(
		"\
INSERT INTO graph_facts (
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
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now(), now())
RETURNING fact_id",
	)
	.bind(Uuid::new_v4())
	.bind(tenant_id)
	.bind(project_id)
	.bind(agent_id)
	.bind(scope)
	.bind(subject_entity_id)
	.bind(predicate)
	.bind(predicate_id)
	.bind(object_entity_id)
	.bind(object_value)
	.bind(valid_from)
	.bind(valid_to)
	.fetch_one(&mut *executor)
	.await?;
	let fact_id = row.0;

	for note_id in evidence_note_ids {
		sqlx::query(
			"\
INSERT INTO graph_fact_evidence (evidence_id, fact_id, note_id, created_at)
VALUES ($1, $2, $3, now())
ON CONFLICT (fact_id, note_id) DO NOTHING",
		)
		.bind(Uuid::new_v4())
		.bind(fact_id)
		.bind(*note_id)
		.execute(&mut *executor)
		.await?;
	}

	Ok(fact_id)
}

#[allow(clippy::too_many_arguments)]
/// Upserts an active graph fact row and ensures the provided evidence links exist.
pub async fn upsert_fact_with_evidence(
	executor: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	scope: &str,
	subject_entity_id: Uuid,
	predicate: &str,
	predicate_id: Uuid,
	object_entity_id: Option<Uuid>,
	object_value: Option<&str>,
	valid_from: OffsetDateTime,
	valid_to: Option<OffsetDateTime>,
	evidence_note_ids: &[Uuid],
) -> Result<Uuid> {
	if evidence_note_ids.is_empty() {
		return Err(Error::InvalidArgument(
			"graph fact evidence is required; evidence_note_ids must not be empty".to_string(),
		));
	}

	let fact_id = match (object_entity_id, object_value) {
		(Some(object_entity_id), None) => {
			let row: (Uuid,) = sqlx::query_as::<_, (Uuid,)>(
				"\
INSERT INTO graph_facts (
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
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now(), now())
ON CONFLICT (tenant_id, project_id, scope, subject_entity_id, predicate_id, object_entity_id)
WHERE valid_to IS NULL AND object_entity_id IS NOT NULL
DO UPDATE
SET updated_at = graph_facts.updated_at
RETURNING fact_id",
			)
			.bind(Uuid::new_v4())
			.bind(tenant_id)
			.bind(project_id)
			.bind(agent_id)
			.bind(scope)
			.bind(subject_entity_id)
			.bind(predicate)
			.bind(predicate_id)
			.bind(Some(object_entity_id))
			.bind(None::<String>)
			.bind(valid_from)
			.bind(valid_to)
			.fetch_one(&mut *executor)
			.await?;

			row.0
		},
		(None, Some(object_value)) => {
			let row: (Uuid,) = sqlx::query_as::<_, (Uuid,)>(
				"\
INSERT INTO graph_facts (
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
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now(), now())
ON CONFLICT (tenant_id, project_id, scope, subject_entity_id, predicate_id, object_value)
WHERE valid_to IS NULL AND object_value IS NOT NULL
DO UPDATE
SET updated_at = graph_facts.updated_at
RETURNING fact_id",
			)
			.bind(Uuid::new_v4())
			.bind(tenant_id)
			.bind(project_id)
			.bind(agent_id)
			.bind(scope)
			.bind(subject_entity_id)
			.bind(predicate)
			.bind(predicate_id)
			.bind(None::<Uuid>)
			.bind(Some(object_value))
			.bind(valid_from)
			.bind(valid_to)
			.fetch_one(&mut *executor)
			.await?;

			row.0
		},
		_ => {
			return Err(Error::InvalidArgument(
				"graph fact must provide exactly one of object_entity_id and object_value"
					.to_string(),
			));
		},
	};

	for note_id in evidence_note_ids {
		sqlx::query(
			"\
INSERT INTO graph_fact_evidence (evidence_id, fact_id, note_id, created_at)
VALUES ($1, $2, $3, now())
ON CONFLICT (fact_id, note_id) DO NOTHING",
		)
		.bind(Uuid::new_v4())
		.bind(fact_id)
		.bind(*note_id)
		.execute(&mut *executor)
		.await?;
	}

	Ok(fact_id)
}

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
