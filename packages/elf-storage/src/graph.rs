use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Error, Result,
	models::{GraphFact, GraphPredicate},
};

const GRAPH_PREDICATE_SCOPE_GLOBAL: &str = "__global__";
const GRAPH_PREDICATE_SCOPE_PROJECT_PREFIX: &str = "__project__:";

pub fn normalize_entity_name(input: &str) -> String {
	input.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

pub fn normalize_predicate_name(input: &str) -> String {
	normalize_entity_name(input)
}

pub async fn resolve_or_register_predicate(
	executor: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	predicate_surface: &str,
) -> Result<GraphPredicate> {
	let predicate_surface = predicate_surface.trim();

	if predicate_surface.is_empty() {
		return Err(Error::InvalidArgument(
			"graph predicate is required; predicate_surface must not be empty".to_string(),
		));
	}

	let alias_norm = normalize_predicate_name(predicate_surface);
	let tenant_project_scope = predicate_scope_key_tenant_project(tenant_id, project_id);
	let project_scope = predicate_scope_key_project(project_id);
	let global_scope = GRAPH_PREDICATE_SCOPE_GLOBAL.to_string();

	for scope_key in [&tenant_project_scope, &project_scope, &global_scope] {
		if let Some(row) = sqlx::query_as::<_, GraphPredicate>(
			"\
SELECT
	gp.predicate_id,
	gp.scope_key,
	gp.tenant_id,
	gp.project_id,
	gp.canonical,
	gp.canonical_norm,
	gp.cardinality,
	gp.status,
	gp.created_at,
	gp.updated_at
FROM graph_predicate_aliases gpa
JOIN graph_predicates gp ON gp.predicate_id = gpa.predicate_id
WHERE gpa.scope_key = $1
	AND gpa.alias_norm = $2
LIMIT 1",
		)
		.bind(scope_key)
		.bind(&alias_norm)
		.fetch_optional(&mut *executor)
		.await?
		{
			return Ok(row);
		}
	}

	let predicate_id = Uuid::new_v4();
	let predicate_row = sqlx::query_as::<_, GraphPredicate>(
		"\
INSERT INTO graph_predicates (
	predicate_id,
	scope_key,
	tenant_id,
	project_id,
	canonical,
	canonical_norm,
	cardinality,
	status,
	created_at,
	updated_at
)
VALUES ($1, $2, $3, $4, $5, $6, 'multi', 'pending', now(), now())
ON CONFLICT (scope_key, canonical_norm)
DO UPDATE
SET canonical = graph_predicates.canonical
RETURNING
	predicate_id,
	scope_key,
	tenant_id,
	project_id,
	canonical,
	canonical_norm,
	cardinality,
	status,
	created_at,
	updated_at",
	)
	.bind(predicate_id)
	.bind(&tenant_project_scope)
	.bind(tenant_id)
	.bind(project_id)
	.bind(predicate_surface)
	.bind(&alias_norm)
	.fetch_one(&mut *executor)
	.await?;

	sqlx::query(
		"\
INSERT INTO graph_predicate_aliases (
	alias_id,
	predicate_id,
	scope_key,
	alias,
	alias_norm,
	created_at
)
VALUES ($1, $2, $3, $4, $5, now())
ON CONFLICT (scope_key, alias_norm) DO NOTHING",
	)
	.bind(Uuid::new_v4())
	.bind(predicate_row.predicate_id)
	.bind(&tenant_project_scope)
	.bind(predicate_surface)
	.bind(&alias_norm)
	.execute(&mut *executor)
	.await?;

	Ok(predicate_row)
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
\tfact_id,
\ttenant_id,
\tproject_id,
\tagent_id,
\tscope,
\tsubject_entity_id,
\tpredicate,
\tpredicate_id,
\tobject_entity_id,
\tobject_value,
\tvalid_from,
\tvalid_to,
\tcreated_at,
\tupdated_at
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
\tfact_id,
\ttenant_id,
\tproject_id,
\tagent_id,
\tscope,
\tsubject_entity_id,
\tpredicate,
\tpredicate_id,
\tobject_entity_id,
\tobject_value,
\tvalid_from,
\tvalid_to,
\tcreated_at,
\tupdated_at
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

fn predicate_scope_key_tenant_project(tenant_id: &str, project_id: &str) -> String {
	format!("{tenant_id}:{project_id}")
}

fn predicate_scope_key_project(project_id: &str) -> String {
	format!("{GRAPH_PREDICATE_SCOPE_PROJECT_PREFIX}{project_id}")
}
