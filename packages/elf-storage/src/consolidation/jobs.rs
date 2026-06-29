use sqlx::PgExecutor;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
	Result, consolidation::types::ConsolidationRunJobInsert, db::Db, models::ConsolidationRunJob,
};

/// Enqueues one consolidation worker job.
pub async fn insert_consolidation_run_job<'e, E>(
	executor: E,
	args: ConsolidationRunJobInsert<'_>,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO consolidation_run_jobs (
	job_id,
	run_id,
	tenant_id,
	project_id,
	agent_id,
	job_kind,
	status,
	payload,
	available_at,
	created_at,
	updated_at
)
VALUES ($1,$2,$3,$4,$5,$6,'PENDING',$7,$8,$8,$8)",
	)
	.bind(args.job_id)
	.bind(args.run_id)
	.bind(args.tenant_id)
	.bind(args.project_id)
	.bind(args.agent_id)
	.bind(args.job_kind)
	.bind(args.payload)
	.bind(args.now)
	.execute(executor)
	.await?;

	Ok(())
}

/// Claims the next due consolidation worker job and leases it until `lease_seconds`.
pub async fn claim_next_consolidation_run_job(
	db: &Db,
	now: OffsetDateTime,
	lease_seconds: i64,
) -> Result<Option<ConsolidationRunJob>> {
	let mut tx = db.pool.begin().await?;
	let row = sqlx::query_as::<_, ConsolidationRunJob>(
		"\
SELECT
	job_id,
	run_id,
	tenant_id,
	project_id,
	agent_id,
	job_kind,
	status,
	payload,
	attempts,
	last_error,
	available_at,
	created_at,
	updated_at
FROM consolidation_run_jobs
WHERE status IN ('PENDING','FAILED','CLAIMED') AND available_at <= $1
ORDER BY available_at ASC
LIMIT 1
FOR UPDATE SKIP LOCKED",
	)
	.bind(now)
	.fetch_optional(&mut *tx)
	.await?;
	let job = if let Some(mut job) = row {
		let lease_until = now + Duration::seconds(lease_seconds);

		sqlx::query(
			"\
UPDATE consolidation_run_jobs
SET status = 'CLAIMED', available_at = $1, updated_at = $2
WHERE job_id = $3",
		)
		.bind(lease_until)
		.bind(now)
		.bind(job.job_id)
		.execute(&mut *tx)
		.await?;

		job.status = "CLAIMED".to_string();
		job.available_at = lease_until;
		job.updated_at = now;

		Some(job)
	} else {
		None
	};

	tx.commit().await?;

	Ok(job)
}

/// Marks a consolidation worker job as completed.
pub async fn mark_consolidation_run_job_done<'e, E>(
	executor: E,
	job_id: Uuid,
	now: OffsetDateTime,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
UPDATE consolidation_run_jobs
SET status = 'DONE', updated_at = $1
WHERE job_id = $2",
	)
	.bind(now)
	.bind(job_id)
	.execute(executor)
	.await?;

	Ok(())
}

/// Marks a consolidation worker job as failed and schedules its retry.
pub async fn mark_consolidation_run_job_failed(
	db: &Db,
	job_id: Uuid,
	attempts: i32,
	error_text: &str,
	available_at: OffsetDateTime,
	now: OffsetDateTime,
) -> Result<()> {
	sqlx::query(
		"\
UPDATE consolidation_run_jobs
SET status = 'FAILED',
	attempts = $1,
	last_error = $2,
	available_at = $3,
	updated_at = $4
WHERE job_id = $5",
	)
	.bind(attempts)
	.bind(error_text)
	.bind(available_at)
	.bind(now)
	.bind(job_id)
	.execute(&db.pool)
	.await?;

	Ok(())
}
