use std::time::{Duration, Instant};

use sqlx::{FromRow, PgPool};
use tokio::time;
use uuid::Uuid;

#[derive(FromRow)]
struct DocOutboxCounts {
	total: i64,
	done: i64,
	failed: i64,
}

#[derive(FromRow)]
struct NoteOutboxCounts {
	total: i64,
	done: i64,
	failed: i64,
}

pub(crate) async fn wait_for_doc_outbox_done(
	pool: &PgPool,
	doc_id: Uuid,
	timeout: Duration,
) -> bool {
	let deadline = Instant::now() + timeout;

	loop {
		let row: Option<DocOutboxCounts> = sqlx::query_as::<_, DocOutboxCounts>(
			"\
SELECT
	COUNT(*) AS total,
	COUNT(*) FILTER (WHERE status = 'DONE') AS done,
	COUNT(*) FILTER (WHERE status = 'FAILED') AS failed
FROM doc_indexing_outbox
WHERE doc_id = $1",
		)
		.bind(doc_id)
		.fetch_optional(pool)
		.await
		.ok()
		.flatten();

		if let Some(row) = row.as_ref()
			&& row.total > 0
			&& row.done == row.total
		{
			return true;
		}
		if let Some(row) = row.as_ref()
			&& row.failed > 0
		{
			return false;
		}

		if Instant::now() >= deadline {
			return false;
		}

		time::sleep(Duration::from_millis(200)).await;
	}
}

pub(crate) async fn wait_for_note_outbox_done(
	pool: &PgPool,
	note_id: Uuid,
	timeout: Duration,
) -> bool {
	let deadline = Instant::now() + timeout;

	loop {
		let row: Option<NoteOutboxCounts> = sqlx::query_as::<_, NoteOutboxCounts>(
			"\
SELECT
	COUNT(*) AS total,
	COUNT(*) FILTER (WHERE status = 'DONE') AS done,
	COUNT(*) FILTER (WHERE status = 'FAILED') AS failed
FROM indexing_outbox
WHERE note_id = $1",
		)
		.bind(note_id)
		.fetch_optional(pool)
		.await
		.ok()
		.flatten();

		if let Some(row) = row.as_ref()
			&& row.total > 0
			&& row.done == row.total
		{
			return true;
		}
		if let Some(row) = row.as_ref()
			&& row.failed > 0
		{
			return false;
		}

		if Instant::now() >= deadline {
			return false;
		}

		time::sleep(Duration::from_millis(200)).await;
	}
}
