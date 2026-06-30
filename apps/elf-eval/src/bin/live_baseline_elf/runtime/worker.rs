use color_eyre::Result;

use crate::{
	Arc, BTreeMap, BaselineRuntime, ElfService, FailedOutboxJob, JoinSet, Uuid, WorkerRunEvidence,
	env, eyre, runtime::service,
};
use elf_worker::worker;

pub(crate) async fn run_worker_until_indexed(
	runtime: &BaselineRuntime,
	service: &ElfService,
	note_ids: &[Uuid],
	label: &str,
) -> Result<WorkerRunEvidence> {
	let concurrency = crate::worker_concurrency();
	let mut states = Vec::with_capacity(concurrency);

	for _ in 0..concurrency {
		states.push(Arc::new(service::build_worker_state(runtime).await?));
	}

	let before = outbox_status_counts(service, note_ids).await?;
	let max_iterations = worker_max_iterations(note_ids.len());
	let mut iterations = 0_usize;

	while iterations < max_iterations {
		let after = outbox_status_counts(service, note_ids).await?;

		if crate::outbox_done(&after, note_ids.len()) {
			let (chunk_rows, chunk_embedding_rows) = chunk_counts(service, note_ids).await?;
			let failed_jobs = failed_outbox_jobs(service, note_ids).await?;

			return Ok(WorkerRunEvidence {
				label: label.to_string(),
				expected_note_count: note_ids.len(),
				concurrency,
				iterations,
				before,
				after,
				chunk_rows,
				chunk_embedding_rows,
				failed_jobs,
			});
		}

		let mut set = JoinSet::new();

		for state in &states {
			let state = Arc::clone(state);

			set.spawn(async move {
				worker::process_once(&state)
					.await
					.map_err(|err| eyre::eyre!("Worker process_once failed: {err}"))
			});
		}

		while let Some(joined) = set.join_next().await {
			joined??;
		}

		iterations = iterations.saturating_add(concurrency);
	}

	let after = outbox_status_counts(service, note_ids).await?;
	let (chunk_rows, chunk_embedding_rows) = chunk_counts(service, note_ids).await?;
	let failed_jobs = failed_outbox_jobs(service, note_ids).await?;

	Ok(WorkerRunEvidence {
		label: label.to_string(),
		expected_note_count: note_ids.len(),
		concurrency,
		iterations,
		before,
		after,
		chunk_rows,
		chunk_embedding_rows,
		failed_jobs,
	})
}

fn worker_max_iterations(note_count: usize) -> usize {
	env::var("ELF_BASELINE_WORKER_MAX_ITERATIONS")
		.ok()
		.and_then(|value| value.parse::<usize>().ok())
		.unwrap_or_else(|| note_count.saturating_mul(3).saturating_add(32))
}

async fn outbox_status_counts(
	service: &ElfService,
	note_ids: &[Uuid],
) -> Result<BTreeMap<String, i64>> {
	if note_ids.is_empty() {
		return Ok(BTreeMap::new());
	}

	let rows = sqlx::query_as::<_, (String, i64)>(
		"\
SELECT status, COUNT(*)::bigint
FROM indexing_outbox
WHERE note_id = ANY($1)
GROUP BY status
ORDER BY status",
	)
	.bind(note_ids)
	.fetch_all(&service.db.pool)
	.await?;

	Ok(rows.into_iter().collect())
}

async fn chunk_counts(service: &ElfService, note_ids: &[Uuid]) -> Result<(i64, i64)> {
	if note_ids.is_empty() {
		return Ok((0, 0));
	}

	let chunk_rows = sqlx::query_scalar::<_, i64>(
		"\
SELECT COUNT(*)::bigint
FROM memory_note_chunks
WHERE note_id = ANY($1)",
	)
	.bind(note_ids)
	.fetch_one(&service.db.pool)
	.await?;
	let chunk_embedding_rows = sqlx::query_scalar::<_, i64>(
		"\
SELECT COUNT(*)::bigint
FROM memory_note_chunks c
JOIN note_chunk_embeddings e ON e.chunk_id = c.chunk_id
WHERE c.note_id = ANY($1)",
	)
	.bind(note_ids)
	.fetch_one(&service.db.pool)
	.await?;

	Ok((chunk_rows, chunk_embedding_rows))
}

async fn failed_outbox_jobs(
	service: &ElfService,
	note_ids: &[Uuid],
) -> Result<Vec<FailedOutboxJob>> {
	if note_ids.is_empty() {
		return Ok(Vec::new());
	}

	let rows = sqlx::query_as::<_, (Uuid, Option<String>, String, i32, Option<String>)>(
		"\
SELECT o.note_id, n.key, o.op, o.attempts, o.last_error
FROM indexing_outbox o
LEFT JOIN memory_notes n ON n.note_id = o.note_id
WHERE o.note_id = ANY($1)
	AND o.status = 'FAILED'
ORDER BY n.key NULLS LAST, o.note_id",
	)
	.bind(note_ids)
	.fetch_all(&service.db.pool)
	.await?;

	Ok(rows
		.into_iter()
		.map(|(note_id, note_key, op, attempts, last_error)| FailedOutboxJob {
			note_id,
			note_key,
			op,
			attempts,
			last_error,
		})
		.collect())
}
