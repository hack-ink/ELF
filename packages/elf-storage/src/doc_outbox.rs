use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Result, db::Db, models::DocIndexingOutboxEntry};

pub async fn enqueue_doc_outbox<'e, E>(
	executor: E,
	doc_id: Uuid,
	chunk_id: Uuid,
	op: &str,
	embedding_version: &str,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO doc_indexing_outbox (outbox_id, doc_id, chunk_id, op, embedding_version, status)
VALUES ($1,$2,$3,$4,$5,'PENDING')",
	)
	.bind(Uuid::new_v4())
	.bind(doc_id)
	.bind(chunk_id)
	.bind(op)
	.bind(embedding_version)
	.execute(executor)
	.await?;

	Ok(())
}

pub async fn claim_next_doc_indexing_outbox_job(
	db: &Db,
	now: OffsetDateTime,
	lease_seconds: i64,
) -> Result<Option<DocIndexingOutboxEntry>> {
	let mut tx = db.pool.begin().await?;
	let row = sqlx::query_as::<_, DocIndexingOutboxEntry>(
		"\
SELECT
\toutbox_id,
\tdoc_id,
\tchunk_id,
\top,
\tembedding_version,
\tstatus,
\tattempts,
\tlast_error,
\tavailable_at,
\tcreated_at,
\tupdated_at
FROM doc_indexing_outbox
WHERE status IN ('PENDING','FAILED','CLAIMED') AND available_at <= $1
ORDER BY available_at ASC
LIMIT 1
FOR UPDATE SKIP LOCKED",
	)
	.bind(now)
	.fetch_optional(&mut *tx)
	.await?;
	let job = if let Some(mut job) = row {
		let lease_until = now + time::Duration::seconds(lease_seconds);

		sqlx::query(
			"UPDATE doc_indexing_outbox SET status = 'CLAIMED', available_at = $1, updated_at = $2 WHERE outbox_id = $3",
		)
		.bind(lease_until)
		.bind(now)
		.bind(job.outbox_id)
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

pub async fn mark_doc_indexing_outbox_done(
	db: &Db,
	outbox_id: Uuid,
	now: OffsetDateTime,
) -> Result<()> {
	sqlx::query(
		"UPDATE doc_indexing_outbox SET status = 'DONE', updated_at = $1 WHERE outbox_id = $2",
	)
	.bind(now)
	.bind(outbox_id)
	.execute(&db.pool)
	.await?;

	Ok(())
}

pub async fn mark_doc_indexing_outbox_failed(
	db: &Db,
	outbox_id: Uuid,
	attempts: i32,
	error_text: &str,
	available_at: OffsetDateTime,
	now: OffsetDateTime,
) -> Result<()> {
	sqlx::query(
		"\
UPDATE doc_indexing_outbox
SET status = 'FAILED',
\tattempts = $1,
\tlast_error = $2,
\tavailable_at = $3,
\tupdated_at = $4
WHERE outbox_id = $5",
	)
	.bind(attempts)
	.bind(error_text)
	.bind(available_at)
	.bind(now)
	.bind(outbox_id)
	.execute(&db.pool)
	.await?;

	Ok(())
}
