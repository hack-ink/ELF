use time::Duration;

use crate::worker::{
	self, OffsetDateTime, POLL_INTERVAL_MS, Result, TRACE_CLEANUP_INTERVAL_SECONDS, WorkerState,
};

/// Runs the worker polling loop for note, document, and trace outboxes.
pub async fn run_worker(state: WorkerState) -> Result<()> {
	let mut last_trace_cleanup = OffsetDateTime::now_utc();

	loop {
		if let Err(err) = worker::process_indexing_outbox_once(&state).await {
			tracing::error!(error = %err, "Indexing outbox processing failed.");
		}
		if let Err(err) = worker::process_doc_indexing_outbox_once(&state).await {
			tracing::error!(error = %err, "Doc indexing outbox processing failed.");
		}
		if let Err(err) = worker::process_trace_outbox_once(&state).await {
			tracing::error!(error = %err, "Search trace outbox processing failed.");
		}
		if let Err(err) = worker::process_consolidation_run_job_once(&state).await {
			tracing::error!(error = %err, "Consolidation run job processing failed.");
		}

		let now = OffsetDateTime::now_utc();

		if now - last_trace_cleanup >= Duration::seconds(TRACE_CLEANUP_INTERVAL_SECONDS) {
			if let Err(err) = worker::purge_expired_trace_candidates(&state.db, now).await {
				tracing::error!(error = %err, "Search trace candidate cleanup failed.");
			}
			if let Err(err) = worker::purge_expired_traces(&state.db, now).await {
				tracing::error!(error = %err, "Search trace cleanup failed.");
			} else {
				last_trace_cleanup = now;
			}
			if let Err(err) = worker::purge_expired_cache(&state.db, now).await {
				tracing::error!(error = %err, "LLM cache cleanup failed.");
			}
			if let Err(err) = worker::purge_expired_search_sessions(&state.db, now).await {
				tracing::error!(error = %err, "Search session cleanup failed.");
			}
		}

		tokio::time::sleep(worker::to_std_duration(Duration::milliseconds(POLL_INTERVAL_MS))).await;
	}
}

/// Processes at most one due job from each worker-owned queue.
pub async fn process_once(state: &WorkerState) -> Result<()> {
	worker::process_indexing_outbox_once(state).await?;
	worker::process_doc_indexing_outbox_once(state).await?;
	worker::process_trace_outbox_once(state).await?;
	worker::process_consolidation_run_job_once(state).await?;

	Ok(())
}
