use sqlx::PgConnection;
use uuid::Uuid;

use crate::{Error, Result, models::GraphPredicate};

use super::query::get_predicate_by_id;

/// Updates a predicate's mutable status and cardinality fields.
pub async fn update_predicate(
	executor: &mut PgConnection,
	predicate_id: Uuid,
	status: Option<&str>,
	cardinality: Option<&str>,
) -> Result<GraphPredicate> {
	let status = status.map(str::trim);

	if status.is_some_and(str::is_empty) {
		return Err(Error::InvalidArgument("graph predicate status must not be empty".to_string()));
	}

	let cardinality = cardinality.map(str::trim);

	if cardinality.is_some_and(str::is_empty) {
		return Err(Error::InvalidArgument(
			"graph predicate cardinality must not be empty".to_string(),
		));
	}

	let row = sqlx::query_as::<_, GraphPredicate>(
		"\
UPDATE graph_predicates
SET
	status = COALESCE($2, status),
	cardinality = COALESCE($3, cardinality),
	updated_at = now()
WHERE predicate_id = $1
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
	.bind(status)
	.bind(cardinality)
	.fetch_optional(&mut *executor)
	.await?;

	row.ok_or_else(|| {
		Error::NotFound(format!("graph predicate not found; predicate_id={predicate_id}"))
	})
}

/// Updates a predicate only when its current state matches the expected guard values.
pub async fn update_predicate_guarded(
	executor: &mut PgConnection,
	predicate_id: Uuid,
	expected_status: &str,
	expected_cardinality: &str,
	status: Option<&str>,
	cardinality: Option<&str>,
) -> Result<GraphPredicate> {
	let expected_status = expected_status.trim();
	let expected_cardinality = expected_cardinality.trim();

	if expected_status.is_empty() {
		return Err(Error::InvalidArgument(
			"graph predicate expected_status must not be empty".to_string(),
		));
	}
	if expected_cardinality.is_empty() {
		return Err(Error::InvalidArgument(
			"graph predicate expected_cardinality must not be empty".to_string(),
		));
	}
	if expected_status == "deprecated" {
		return Err(Error::Conflict(format!(
			"graph predicate is deprecated and cannot be modified; predicate_id={predicate_id}"
		)));
	}

	let status = status.map(str::trim);

	if status.is_some_and(str::is_empty) {
		return Err(Error::InvalidArgument("graph predicate status must not be empty".to_string()));
	}

	let cardinality = cardinality.map(str::trim);

	if cardinality.is_some_and(str::is_empty) {
		return Err(Error::InvalidArgument(
			"graph predicate cardinality must not be empty".to_string(),
		));
	}

	let row = sqlx::query_as::<_, GraphPredicate>(
		"\
	UPDATE graph_predicates
	SET
		status = COALESCE($4, status),
		cardinality = COALESCE($5, cardinality),
		updated_at = now()
	WHERE predicate_id = $1
		AND status = $2
		AND cardinality = $3
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
	.bind(expected_status)
	.bind(expected_cardinality)
	.bind(status)
	.bind(cardinality)
	.fetch_optional(&mut *executor)
	.await?;

	if let Some(row) = row {
		return Ok(row);
	}

	let existing = get_predicate_by_id(executor, predicate_id).await?;
	let Some(_) = existing else {
		return Err(Error::NotFound(format!(
			"graph predicate not found; predicate_id={predicate_id}"
		)));
	};

	Err(Error::Conflict(format!(
		"graph predicate update conflict; predicate_id={predicate_id} expected_status={expected_status} expected_cardinality={expected_cardinality}"
	)))
}
