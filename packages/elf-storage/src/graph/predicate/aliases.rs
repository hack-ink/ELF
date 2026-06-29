use sqlx::PgConnection;
use uuid::Uuid;

use crate::{Error, Result, graph, models::GraphPredicateAlias};

/// Registers an additional alias for an existing predicate.
pub async fn add_predicate_alias(
	executor: &mut PgConnection,
	predicate_id: Uuid,
	alias: &str,
) -> Result<()> {
	let alias = alias.trim();

	if alias.is_empty() {
		return Err(Error::InvalidArgument(
			"graph predicate alias is required; alias must not be empty".to_string(),
		));
	}

	let alias_norm = graph::normalize_predicate_name(alias);

	if alias_norm.is_empty() {
		return Err(Error::InvalidArgument(
			"graph predicate alias is required; alias_norm must not be empty".to_string(),
		));
	}

	let predicate_scope_key: Option<(String,)> = sqlx::query_as(
		"\
SELECT scope_key
FROM graph_predicates
WHERE predicate_id = $1",
	)
	.bind(predicate_id)
	.fetch_optional(&mut *executor)
	.await?;
	let Some((scope_key,)) = predicate_scope_key else {
		return Err(Error::NotFound(format!(
			"graph predicate not found; predicate_id={predicate_id}"
		)));
	};
	let res = sqlx::query(
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
ON CONFLICT (scope_key, alias_norm) DO UPDATE
SET alias = EXCLUDED.alias
WHERE graph_predicate_aliases.predicate_id = EXCLUDED.predicate_id",
	)
	.bind(Uuid::new_v4())
	.bind(predicate_id)
	.bind(&scope_key)
	.bind(alias)
	.bind(&alias_norm)
	.execute(&mut *executor)
	.await?;

	if res.rows_affected() == 0 {
		return Err(Error::Conflict(format!(
			"graph predicate alias already bound; scope_key={scope_key} alias_norm={alias_norm}"
		)));
	}

	Ok(())
}

/// Lists aliases bound to one predicate.
pub async fn list_predicate_aliases(
	executor: &mut PgConnection,
	predicate_id: Uuid,
) -> Result<Vec<GraphPredicateAlias>> {
	let rows = sqlx::query_as::<_, GraphPredicateAlias>(
		"\
SELECT
	alias_id,
	predicate_id,
	scope_key,
	alias,
	alias_norm,
	created_at
FROM graph_predicate_aliases
WHERE predicate_id = $1
ORDER BY created_at ASC, alias_norm ASC",
	)
	.bind(predicate_id)
	.fetch_all(&mut *executor)
	.await?;

	Ok(rows)
}
