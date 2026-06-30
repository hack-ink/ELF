use color_eyre::Result;

use crate::checks::{
	self, Arc, BaselineRuntime, CheckResult, Duration, ElfService, Instant, JoinSet, Report, Uuid,
	eyre, time,
};

pub(crate) async fn run_concurrent_write_check_impl(
	runtime: &BaselineRuntime,
	service: Arc<ElfService>,
) -> Result<CheckResult> {
	let note_count = checks::concurrent_note_count();
	let mut set = JoinSet::new();

	for index in 0..note_count {
		let request = checks::concurrent_add_request(index);
		let service_ref = Arc::clone(&service);

		set.spawn(async move {
			let response = service_ref.add_note(request).await?;
			let note_id = response
				.results
				.first()
				.and_then(|result| result.note_id)
				.ok_or_else(|| eyre::eyre!("Concurrent add_note did not return a note_id."))?;

			Ok::<Uuid, Report>(note_id)
		});
	}

	let mut note_ids = Vec::with_capacity(note_count);

	while let Some(joined) = set.join_next().await {
		note_ids.push(joined??);
	}

	let worker_evidence =
		checks::run_worker_until_indexed(runtime, &service, &note_ids, "concurrent_upsert").await?;
	let probe_indexes = checks::concurrency_probe_indexes(note_count);
	let mut query_results = Vec::new();

	for index in probe_indexes {
		query_results
			.push(checks::run_single_query(&service, checks::concurrent_query_case(index)).await?);
	}

	let pass_count = query_results.iter().filter(|result| result.matched).count();
	let pass = checks::outbox_done(&worker_evidence.after, worker_evidence.expected_note_count)
		&& pass_count == query_results.len();

	Ok(CheckResult {
		name: "concurrent_write_search_e2e",
		status: if pass { "pass" } else { "lifecycle_fail" },
		reason: if pass {
			"Concurrent add_note calls were indexed by the worker and remained searchable."
				.to_string()
		} else {
			"Concurrent add_note calls did not all become searchable after worker indexing."
				.to_string()
		},
		evidence: serde_json::json!({
			"note_count": note_count,
			"worker": worker_evidence,
			"query_summary": {
				"total": query_results.len(),
				"pass": pass_count,
				"fail": query_results.len().saturating_sub(pass_count),
			},
			"queries": query_results,
		}),
	})
}

pub(crate) async fn run_soak_stability_check_impl(
	runtime: &BaselineRuntime,
	service: Arc<ElfService>,
) -> Result<Option<CheckResult>> {
	let config = checks::soak_config();

	if config.target_seconds == 0 && config.write_rounds == 0 {
		return Ok(None);
	}

	let target_duration = Duration::from_secs(config.target_seconds);
	let started_at = Instant::now();
	let write_rounds = config.write_rounds.max(if config.target_seconds > 0 { 1 } else { 0 });
	let mut note_ids = Vec::with_capacity(write_rounds);
	let mut worker_runs = Vec::with_capacity(write_rounds);
	let mut query_results = Vec::new();

	for index in 0..write_rounds {
		let response = service.add_note(checks::soak_add_request(index)).await?;
		let note_id = response
			.results
			.first()
			.and_then(|result| result.note_id)
			.ok_or_else(|| eyre::eyre!("Soak add_note did not return a note_id."))?;

		note_ids.push(note_id);
		worker_runs.push(
			checks::run_worker_until_indexed(runtime, &service, &[note_id], "soak_upsert").await?,
		);
		query_results
			.push(checks::run_single_query(&service, checks::soak_query_case(index)).await?);

		if config.target_seconds > 0 && write_rounds > 1 {
			let target_elapsed = target_duration.mul_f64((index + 1) as f64 / write_rounds as f64);

			if started_at.elapsed() < target_elapsed {
				time::sleep(target_elapsed.saturating_sub(started_at.elapsed())).await;
			}
		}
	}

	let mut probe_index = 0;

	while started_at.elapsed() < target_duration {
		let index = probe_index % write_rounds;

		query_results
			.push(checks::run_single_query(&service, checks::soak_query_case(index)).await?);

		probe_index += 1;

		let sleep_for = Duration::from_millis(config.probe_interval_millis)
			.min(target_duration.saturating_sub(started_at.elapsed()));

		if !sleep_for.is_zero() {
			time::sleep(sleep_for).await;
		}
	}

	let elapsed_seconds = started_at.elapsed().as_secs_f64();
	let pass_count = query_results.iter().filter(|result| result.matched).count();
	let query_fail_count = query_results.len().saturating_sub(pass_count);
	let worker_pass =
		worker_runs.iter().all(|run| checks::outbox_done(&run.after, run.expected_note_count));
	let duration_pass = target_duration.is_zero() || started_at.elapsed() >= target_duration;
	let pass = worker_pass && duration_pass && query_fail_count == 0;
	let failed_queries = query_results.iter().filter(|result| !result.matched).collect::<Vec<_>>();

	Ok(Some(CheckResult {
		name: "soak_stability_e2e",
		status: if pass { "pass" } else { "lifecycle_fail" },
		reason: if pass {
			"ELF sustained repeated write, worker indexing, and search probes for the configured soak window.".to_string()
		} else {
			"ELF did not sustain the configured soak write/search window without a failed worker or retrieval probe.".to_string()
		},
		evidence: serde_json::json!({
			"config": config,
			"elapsed_seconds": elapsed_seconds,
			"duration_met": duration_pass,
			"worker_pass": worker_pass,
			"write_note_ids": note_ids,
			"worker_runs": worker_runs,
			"query_summary": {
				"total": query_results.len(),
				"pass": pass_count,
				"fail": query_fail_count,
			},
			"failed_queries": failed_queries,
		}),
	}))
}
