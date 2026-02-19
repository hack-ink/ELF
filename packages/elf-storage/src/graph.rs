use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Result, models::GraphFact};

pub fn normalize_entity_name(input: &str) -> String {
	input.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_fact_with_evidence(
	executor: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	scope: &str,
	subject_entity_id: Uuid,
	predicate: &str,
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
	object_entity_id,
	object_value,
	valid_from,
	valid_to,
	created_at,
	updated_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now(), now())
RETURNING fact_id",
	)
	.bind(Uuid::new_v4())
	.bind(tenant_id)
	.bind(project_id)
	.bind(agent_id)
	.bind(scope)
	.bind(subject_entity_id)
	.bind(predicate)
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
pub async fn upsert_fact_with_evidence(
	executor: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	scope: &str,
	subject_entity_id: Uuid,
	predicate: &str,
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
\tfact_id,
\ttenant_id,
\tproject_id,
\tagent_id,
\tscope,
\tsubject_entity_id,
\tpredicate,
\tobject_entity_id,
\tobject_value,
\tvalid_from,
\tvalid_to,
\tcreated_at,
\tupdated_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now(), now())
ON CONFLICT (tenant_id, project_id, scope, subject_entity_id, predicate, object_entity_id)
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
\tfact_id,
\ttenant_id,
\tproject_id,
\tagent_id,
\tscope,
\tsubject_entity_id,
\tpredicate,
\tobject_entity_id,
\tobject_value,
\tvalid_from,
\tvalid_to,
\tcreated_at,
\tupdated_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now(), now())
ON CONFLICT (tenant_id, project_id, scope, subject_entity_id, predicate, object_value)
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

pub async fn upsert_entity(
	executor: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	canonical: &str,
	kind: Option<&str>,
) -> Result<Uuid> {
	let canonical_norm = normalize_entity_name(canonical);
	let row: (Uuid,) = sqlx::query_as(
		"\
INSERT INTO graph_entities (
	entity_id,
	tenant_id,
	project_id,
	canonical,
	canonical_norm,
	kind,
	created_at,
	updated_at
)
VALUES (
	$1, $2, $3, $4, $5, $6, now(), now()
)
ON CONFLICT (tenant_id, project_id, canonical_norm)
DO UPDATE
SET
	canonical = EXCLUDED.canonical,
	kind = COALESCE(EXCLUDED.kind, graph_entities.kind),
	updated_at = now()
RETURNING entity_id",
	)
	.bind(Uuid::new_v4())
	.bind(tenant_id)
	.bind(project_id)
	.bind(canonical)
	.bind(&canonical_norm)
	.bind(kind)
	.fetch_one(executor)
	.await?;

	Ok(row.0)
}

pub async fn upsert_entity_alias(
	executor: &mut PgConnection,
	entity_id: Uuid,
	alias: &str,
) -> Result<()> {
	let alias_norm = normalize_entity_name(alias);

	sqlx::query(
		"\
INSERT INTO graph_entity_aliases (
	alias_id,
	entity_id,
	alias,
	alias_norm,
	created_at
)
VALUES ($1, $2, $3, $4, now())
ON CONFLICT (entity_id, alias_norm)
DO UPDATE SET alias = EXCLUDED.alias",
	)
	.bind(Uuid::new_v4())
	.bind(entity_id)
	.bind(alias)
	.bind(&alias_norm)
	.execute(executor)
	.await?;

	Ok(())
}

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
