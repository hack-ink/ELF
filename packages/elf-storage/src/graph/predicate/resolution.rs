use sqlx::PgConnection;
use uuid::Uuid;

use crate::{
	Error, Result,
	graph::{self, GRAPH_PREDICATE_SCOPE_GLOBAL},
	models::GraphPredicate,
};

/// Resolves a predicate surface across visible scopes or registers a project-scoped predicate.
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

	let alias_norm = graph::normalize_predicate_name(predicate_surface);
	let tenant_project_scope = graph::predicate_scope_key_tenant_project(tenant_id, project_id);
	let project_scope = graph::predicate_scope_key_project(project_id);
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

/// Resolves a predicate surface across visible scopes without creating a new predicate.
pub async fn resolve_predicate_no_register(
	executor: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	predicate_surface: &str,
) -> Result<Option<GraphPredicate>> {
	let predicate_surface = predicate_surface.trim();

	if predicate_surface.is_empty() {
		return Err(Error::InvalidArgument(
			"graph predicate is required; predicate_surface must not be empty".to_string(),
		));
	}

	let alias_norm = graph::normalize_predicate_name(predicate_surface);
	let tenant_project_scope = graph::predicate_scope_key_tenant_project(tenant_id, project_id);
	let project_scope = graph::predicate_scope_key_project(project_id);
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
			return Ok(Some(row));
		}
	}

	Ok(None)
}
