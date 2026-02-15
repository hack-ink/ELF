use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	Result,
	db::Db,
	models::{IndexingOutboxEntry, TraceOutboxJob},
};

pub async fn enqueue_outbox<'e, E>(
	executor: E,
	note_id: Uuid,
	op: &str,
	embedding_version: &str,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query!(
		"INSERT INTO indexing_outbox (outbox_id, note_id, op, embedding_version, status) \
VALUES ($1,$2,$3,$4,'PENDING')",
		Uuid::new_v4(),
		note_id,
		op,
		embedding_version,
	)
	.execute(executor)
	.await?;

	Ok(())
}

pub async fn claim_next_indexing_outbox_job(
	db: &Db,
	now: OffsetDateTime,
	lease_seconds: i64,
) -> Result<Option<IndexingOutboxEntry>> {
	let mut tx = db.pool.begin().await?;
	let row = sqlx::query_as!(
		IndexingOutboxEntry,
		"\
SELECT
	outbox_id,
	note_id,
	op,
	embedding_version,
	status,
	attempts,
	last_error,
	available_at,
	created_at,
	updated_at
FROM indexing_outbox
WHERE status IN ('PENDING','FAILED') AND available_at <= $1
ORDER BY available_at ASC
LIMIT 1
FOR UPDATE SKIP LOCKED",
		now,
	)
	.fetch_optional(&mut *tx)
	.await?;
	let job = if let Some(mut job) = row {
		let lease_until = now + time::Duration::seconds(lease_seconds);

		sqlx::query!(
			"UPDATE indexing_outbox SET available_at = $1, updated_at = $2 WHERE outbox_id = $3",
			lease_until,
			now,
			job.outbox_id,
		)
		.execute(&mut *tx)
		.await?;

		job.available_at = lease_until;
		job.updated_at = now;

		Some(job)
	} else {
		None
	};

	tx.commit().await?;

	Ok(job)
}

pub async fn mark_indexing_outbox_done(
	db: &Db,
	outbox_id: Uuid,
	now: OffsetDateTime,
) -> Result<()> {
	sqlx::query!(
		"UPDATE indexing_outbox SET status = 'DONE', updated_at = $1 WHERE outbox_id = $2",
		now,
		outbox_id,
	)
	.execute(&db.pool)
	.await?;

	Ok(())
}

pub async fn mark_indexing_outbox_failed(
	db: &Db,
	outbox_id: Uuid,
	attempts: i32,
	error_text: &str,
	available_at: OffsetDateTime,
	now: OffsetDateTime,
) -> Result<()> {
	sqlx::query!(
		"\
UPDATE indexing_outbox
SET status = 'FAILED',
	attempts = $1,
	last_error = $2,
	available_at = $3,
	updated_at = $4
WHERE outbox_id = $5",
		attempts,
		error_text,
		available_at,
		now,
		outbox_id,
	)
	.execute(&db.pool)
	.await?;

	Ok(())
}

pub async fn claim_next_trace_outbox_job(
	db: &Db,
	now: OffsetDateTime,
	lease_seconds: i64,
) -> Result<Option<TraceOutboxJob>> {
	let mut tx = db.pool.begin().await?;
	let row = sqlx::query_as!(
		TraceOutboxJob,
		"\
SELECT
	outbox_id,
	trace_id,
	payload,
	attempts
FROM search_trace_outbox
WHERE status IN ('PENDING','FAILED') AND available_at <= $1
ORDER BY available_at ASC
LIMIT 1
FOR UPDATE SKIP LOCKED",
		now,
	)
	.fetch_optional(&mut *tx)
	.await?;
	let job = if let Some(job) = row {
		let lease_until = now + time::Duration::seconds(lease_seconds);

		sqlx::query!(
			"UPDATE search_trace_outbox SET available_at = $1, updated_at = $2 WHERE outbox_id = $3",
			lease_until,
			now,
			job.outbox_id,
		)
		.execute(&mut *tx)
		.await?;

		Some(job)
	} else {
		None
	};

	tx.commit().await?;

	Ok(job)
}

pub async fn mark_trace_outbox_done(db: &Db, outbox_id: Uuid, now: OffsetDateTime) -> Result<()> {
	sqlx::query!(
		"UPDATE search_trace_outbox SET status = 'DONE', updated_at = $1 WHERE outbox_id = $2",
		now,
		outbox_id,
	)
	.execute(&db.pool)
	.await?;

	Ok(())
}

pub async fn mark_trace_outbox_failed(
	db: &Db,
	outbox_id: Uuid,
	attempts: i32,
	error_text: &str,
	available_at: OffsetDateTime,
	now: OffsetDateTime,
) -> Result<()> {
	sqlx::query!(
		"\
UPDATE search_trace_outbox
SET status = 'FAILED',
	attempts = $1,
	last_error = $2,
	available_at = $3,
	updated_at = $4
WHERE outbox_id = $5",
		attempts,
		error_text,
		available_at,
		now,
		outbox_id,
	)
	.execute(&db.pool)
	.await?;

	Ok(())
}
