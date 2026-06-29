use sqlx::PgConnection;
use uuid::Uuid;

use crate::{Result, models::GraphPredicate};

/// Lists predicates visible within the provided scope keys.
pub async fn list_predicates_by_scope_keys(
	executor: &mut PgConnection,
	scope_keys: &[String],
) -> Result<Vec<GraphPredicate>> {
	if scope_keys.is_empty() {
		return Ok(vec![]);
	}

	let scope_keys = scope_keys.to_vec();
	let rows = sqlx::query_as::<_, GraphPredicate>(
		"\
SELECT
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
FROM graph_predicates
WHERE scope_key = ANY($1::text[])
ORDER BY scope_key, canonical_norm",
	)
	.bind(&scope_keys)
	.fetch_all(&mut *executor)
	.await?;

	Ok(rows)
}

/// Fetches one predicate by identifier.
pub async fn get_predicate_by_id(
	executor: &mut PgConnection,
	predicate_id: Uuid,
) -> Result<Option<GraphPredicate>> {
	let row = sqlx::query_as::<_, GraphPredicate>(
		"\
SELECT
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
FROM graph_predicates
WHERE predicate_id = $1",
	)
	.bind(predicate_id)
	.fetch_optional(&mut *executor)
	.await?;

	Ok(row)
}
