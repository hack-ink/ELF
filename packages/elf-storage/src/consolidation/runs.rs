use sqlx::PgExecutor;
use uuid::Uuid;

use super::{sql::CONSOLIDATION_RUN_SELECT, types::ConsolidationRunStateUpdate};
use crate::{Result, models::ConsolidationRun};

/// Inserts one consolidation run.
pub async fn insert_consolidation_run<'e, E>(executor: E, run: &ConsolidationRun) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO consolidation_runs (
	run_id,
	tenant_id,
	project_id,
	agent_id,
	contract_schema,
	job_kind,
	status,
	input_refs,
	source_snapshot,
	lineage,
	error,
	created_at,
	updated_at,
	completed_at
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
	)
	.bind(run.run_id)
	.bind(run.tenant_id.as_str())
	.bind(run.project_id.as_str())
	.bind(run.agent_id.as_str())
	.bind(run.contract_schema.as_str())
	.bind(run.job_kind.as_str())
	.bind(run.status.as_str())
	.bind(&run.input_refs)
	.bind(&run.source_snapshot)
	.bind(&run.lineage)
	.bind(&run.error)
	.bind(run.created_at)
	.bind(run.updated_at)
	.bind(run.completed_at)
	.execute(executor)
	.await?;

	Ok(())
}

/// Fetches one consolidation run by tenant and run identifier.
pub async fn get_consolidation_run<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	run_id: Uuid,
) -> Result<Option<ConsolidationRun>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, ConsolidationRun>(CONSOLIDATION_RUN_SELECT)
		.bind(tenant_id)
		.bind(project_id)
		.bind(run_id)
		.fetch_optional(executor)
		.await?;

	Ok(row)
}

/// Lists consolidation runs for one tenant and project.
pub async fn list_consolidation_runs<'e, E>(
	executor: E,
	tenant_id: &str,
	project_id: &str,
	limit: i64,
) -> Result<Vec<ConsolidationRun>>
where
	E: PgExecutor<'e>,
{
	let rows = sqlx::query_as::<_, ConsolidationRun>(
		"\
SELECT
	run_id,
	tenant_id,
	project_id,
	agent_id,
	contract_schema,
	job_kind,
	status,
	input_refs,
	source_snapshot,
	lineage,
	COALESCE(error, '{}'::jsonb) AS error,
	created_at,
	updated_at,
	completed_at
FROM consolidation_runs
WHERE tenant_id = $1 AND project_id = $2
ORDER BY created_at DESC, run_id DESC
LIMIT $3",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(limit)
	.fetch_all(executor)
	.await?;

	Ok(rows)
}

/// Updates one consolidation run state.
pub async fn update_consolidation_run_state<'e, E>(
	executor: E,
	args: ConsolidationRunStateUpdate<'_>,
) -> Result<Option<ConsolidationRun>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, ConsolidationRun>(
		"\
UPDATE consolidation_runs
SET
	status = $1,
	error = $2,
	updated_at = $3,
	completed_at = CASE
		WHEN $1 IN ('completed', 'failed', 'cancelled') THEN $3
		ELSE completed_at
	END
WHERE tenant_id = $4 AND project_id = $5 AND run_id = $6
RETURNING
	run_id,
	tenant_id,
	project_id,
	agent_id,
	contract_schema,
	job_kind,
	status,
	input_refs,
	source_snapshot,
	lineage,
	COALESCE(error, '{}'::jsonb) AS error,
	created_at,
	updated_at,
	completed_at",
	)
	.bind(args.status)
	.bind(args.error)
	.bind(args.now)
	.bind(args.tenant_id)
	.bind(args.project_id)
	.bind(args.run_id)
	.fetch_optional(executor)
	.await?;

	Ok(row)
}
