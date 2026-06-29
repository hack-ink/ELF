use crate::worker::{Db, OffsetDateTime, Result};

pub(in crate::worker) async fn purge_expired_trace_candidates(
	db: &Db,
	now: OffsetDateTime,
) -> Result<()> {
	let result = sqlx::query("DELETE FROM search_trace_candidates WHERE expires_at <= $1")
		.bind(now)
		.execute(&db.pool)
		.await?;

	if result.rows_affected() > 0 {
		tracing::info!(count = result.rows_affected(), "Purged expired search trace candidates.");
	}

	Ok(())
}

pub(in crate::worker) async fn purge_expired_traces(db: &Db, now: OffsetDateTime) -> Result<()> {
	let result = sqlx::query("DELETE FROM search_traces WHERE expires_at <= $1")
		.bind(now)
		.execute(&db.pool)
		.await?;

	if result.rows_affected() > 0 {
		tracing::info!(count = result.rows_affected(), "Purged expired search traces.");
	}

	Ok(())
}

pub(in crate::worker) async fn purge_expired_cache(db: &Db, now: OffsetDateTime) -> Result<()> {
	let result = sqlx::query("DELETE FROM llm_cache WHERE expires_at <= $1")
		.bind(now)
		.execute(&db.pool)
		.await?;

	if result.rows_affected() > 0 {
		tracing::info!(count = result.rows_affected(), "Purged expired LLM cache entries.");
	}

	Ok(())
}

pub(in crate::worker) async fn purge_expired_search_sessions(
	db: &Db,
	now: OffsetDateTime,
) -> Result<()> {
	let result = sqlx::query("DELETE FROM search_sessions WHERE expires_at <= $1")
		.bind(now)
		.execute(&db.pool)
		.await?;

	if result.rows_affected() > 0 {
		tracing::info!(count = result.rows_affected(), "Purged expired search sessions.");
	}

	Ok(())
}
