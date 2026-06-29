#[path = "checks/lifecycle.rs"] mod lifecycle;
#[path = "checks/reporting.rs"] mod reporting;
#[path = "checks/resource.rs"] mod resource;
#[path = "checks/stress.rs"] mod stress;

use color_eyre::Result;

use crate::{
	AGENT_ID, AddNoteInput, AddNoteRequest, Arc, BTreeMap, BackfillReport, BaselineRuntime,
	CheckResult, CheckSummary, CorpusNote, CostProxyReport, DeleteRequest, Duration, ElfService,
	EmbeddingRuntimeReport, Instant, JoinSet, OperationalCase, PROJECT_ID, Path, QueryCase,
	QueryResult, Report, ResourceEnvelopeEvidence, SCOPE, SoakConfig, TENANT_ID, UpdateRequest,
	Uuid, WorkerRunEvidence, contains_case_insensitive, distinctive_terms, env, eyre, fs,
	run_single_query,
	runtime::{build_service, run_worker_until_indexed},
	time,
};

pub(super) fn outbox_done(counts: &BTreeMap<String, i64>, expected_note_count: usize) -> bool {
	let done = counts.get("DONE").copied().unwrap_or_default();
	let expected = i64::try_from(expected_note_count).unwrap_or(i64::MAX);
	let pending = counts.get("PENDING").copied().unwrap_or_default();
	let failed = counts.get("FAILED").copied().unwrap_or_default();
	let claimed = counts.get("CLAIMED").copied().unwrap_or_default();

	done >= expected && pending == 0 && failed == 0 && claimed == 0
}

pub(super) fn retrieval_check(query_results: &[QueryResult]) -> CheckResult {
	let pass_count = query_results.iter().filter(|result| result.matched).count();
	let fail_count = query_results.len().saturating_sub(pass_count);
	let expected_evidence_ids = query_results
		.iter()
		.map(|result| {
			serde_json::json!({
				"query_id": result.id,
				"expected": result.expected_evidence_ids,
				"allowed_alternates": result.allowed_alternate_evidence_ids,
			})
		})
		.collect::<Vec<_>>();

	CheckResult {
		name: "same_corpus_retrieval",
		status: if fail_count == 0 { "pass" } else { "wrong_result" },
		reason: if fail_count == 0 {
			"All same-corpus retrieval queries returned expected evidence.".to_string()
		} else {
			format!("{fail_count} same-corpus retrieval query case(s) missed expected evidence.")
		},
		evidence: serde_json::json!({
			"total": query_results.len(),
			"pass": pass_count,
			"fail": fail_count,
			"wrong_result_count": fail_count,
			"expected_evidence_ids": expected_evidence_ids,
		}),
	}
}

pub(super) fn worker_indexing_check(evidence: WorkerRunEvidence) -> CheckResult {
	let pass = outbox_done(&evidence.after, evidence.expected_note_count)
		&& evidence.chunk_rows >= i64::try_from(evidence.expected_note_count).unwrap_or(i64::MAX)
		&& evidence.chunk_embedding_rows >= evidence.chunk_rows;

	CheckResult {
		name: "async_worker_indexing_e2e",
		status: if pass { "pass" } else { "lifecycle_fail" },
		reason: if pass {
			"ELF worker processed corpus outbox jobs into persisted chunks and embeddings."
				.to_string()
		} else {
			"ELF worker did not fully process corpus outbox jobs into searchable chunks."
				.to_string()
		},
		evidence: serde_json::json!(evidence),
	}
}

pub(super) fn resumable_backfill_check(report: &BackfillReport) -> CheckResult {
	let resume_pass = !report.resume.enabled
		|| (report.resume.interrupted
			&& report.resume.resume_attempts >= 2
			&& report.skipped_completed > 0);
	let pass = report.completed_count == report.source_count
		&& report.duplicate_source_notes.is_empty()
		&& resume_pass;

	CheckResult {
		name: "resumable_backfill_no_duplicates",
		status: if pass { "pass" } else { "lifecycle_fail" },
		reason: if pass {
			"Checkpointed backfill resumed from durable progress and did not duplicate source documents."
				.to_string()
		} else {
			"Checkpointed backfill did not complete cleanly, did not prove resume, or duplicated source documents."
				.to_string()
		},
		evidence: serde_json::json!(report),
	}
}

pub(super) fn concurrent_note_count() -> usize {
	if let Ok(value) = env::var("ELF_BASELINE_CONCURRENT_NOTES")
		&& let Ok(parsed) = value.parse::<usize>()
	{
		return parsed.max(1);
	}

	match env::var("ELF_BASELINE_PROFILE").as_deref() {
		Ok("backfill" | "large") => 32,
		Ok("stress") => 32,
		Ok("scale" | "full") => 16,
		_ => 4,
	}
}

pub(super) fn concurrent_add_request(index: usize) -> AddNoteRequest {
	let marker = concurrent_marker(index);

	AddNoteRequest {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		agent_id: AGENT_ID.to_string(),
		scope: SCOPE.to_string(),
		notes: vec![AddNoteInput {
			r#type: "fact".to_string(),
			key: Some(format!("concurrent_{index:03}")),
			text: format!(
				"Concurrent benchmark note {index:03} records marker `{marker}` for write race validation."
			),
			structured: None,
			importance: 0.91,
			confidence: 0.96,
			ttl_days: None,
			source_ref: serde_json::json!({
				"source": "ELF live baseline concurrent write check",
				"document": format!("concurrent-{index:03}.md"),
			}),
			write_policy: None,
		}],
	}
}

pub(super) fn concurrent_query_case(index: usize) -> QueryCase {
	let marker = concurrent_marker(index);

	QueryCase::generated(
		format!("concurrent-{index:03}"),
		format!("Find the concurrent benchmark note containing marker {marker}."),
		format!("concurrent-{index:03}.md"),
		vec![marker],
	)
}

pub(super) fn concurrent_marker(index: usize) -> String {
	format!("concurrency-{}-{index:03}", marker_word(index))
}

pub(super) fn soak_config() -> SoakConfig {
	let profile = env::var("ELF_BASELINE_PROFILE").ok();
	let (default_seconds, default_rounds) = match profile.as_deref() {
		Some("backfill" | "large") => (60, 6),
		Some("stress") => (60, 6),
		Some("scale" | "full") => (15, 3),
		_ => (0, 0),
	};

	SoakConfig {
		target_seconds: parse_env_u64("ELF_BASELINE_SOAK_SECONDS").unwrap_or(default_seconds),
		write_rounds: parse_env_usize("ELF_BASELINE_SOAK_ROUNDS").unwrap_or(default_rounds),
		probe_interval_millis: parse_env_u64("ELF_BASELINE_SOAK_PROBE_INTERVAL_MS")
			.unwrap_or(1_000)
			.max(100),
	}
}

pub(super) fn parse_env_u64(name: &str) -> Option<u64> {
	env::var(name).ok()?.parse::<u64>().ok()
}

pub(super) fn parse_env_usize(name: &str) -> Option<usize> {
	env::var(name).ok()?.parse::<usize>().ok()
}

pub(super) fn soak_add_request(index: usize) -> AddNoteRequest {
	let marker = soak_marker(index);
	let (topic, detail) = soak_topic(index);

	AddNoteRequest {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		agent_id: AGENT_ID.to_string(),
		scope: SCOPE.to_string(),
		notes: vec![AddNoteInput {
			r#type: "fact".to_string(),
			key: Some(format!("soak_{index:03}")),
			text: format!(
				"Soak benchmark note {index:03} covers {topic}. {detail} It records stability marker `{marker}` for repeated worker and search probes."
			),
			structured: None,
			importance: 0.92,
			confidence: 0.97,
			ttl_days: None,
			source_ref: serde_json::json!({
				"source": "ELF live baseline soak stability check",
				"document": format!("soak-{index:03}.md"),
			}),
			write_policy: None,
		}],
	}
}

pub(super) fn soak_query_case(index: usize) -> QueryCase {
	let marker = soak_marker(index);
	let (topic, _) = soak_topic(index);

	QueryCase::generated(
		format!("soak-{index:03}"),
		format!("Find the soak benchmark note about {topic} containing marker {marker}."),
		format!("soak-{index:03}.md"),
		vec![marker],
	)
}

pub(super) fn soak_marker(index: usize) -> String {
	format!("soak-stability-{}-{index:03}", marker_word(index))
}

pub(super) fn marker_word(index: usize) -> &'static str {
	const WORDS: &[&str] = &[
		"aurora", "banyan", "cobalt", "delta", "ember", "fennel", "granite", "harbor", "indigo",
		"jasper", "keystone", "lantern", "meridian", "nebula", "onyx", "prairie", "quartz",
		"raven", "solstice", "topaz", "umbra", "verdant", "willow", "xenon", "yarrow", "zephyr",
		"atlas", "beacon", "citadel", "drift", "equinox", "forge",
	];

	WORDS[index % WORDS.len()]
}

pub(super) fn soak_topic(index: usize) -> (&'static str, &'static str) {
	const TOPICS: &[(&str, &str)] = &[
		(
			"release rollback fencing",
			"The rollback controller waits for a signed deploy fence before the next canary.",
		),
		(
			"invoice export batching",
			"The exporter groups invoice CSV rows by merchant ledger before upload.",
		),
		("search shard warming", "The search router warms tenant shard caches before rank probes."),
		(
			"incident pager routing",
			"The incident desk routes page ownership through the release captain.",
		),
		(
			"backup restore rehearsal",
			"The restore rehearsal checks WAL freshness before dry-run recovery.",
		),
		(
			"feature flag expiry",
			"The flag sweeper archives expired toggles before deleting rollout rules.",
		),
		(
			"support queue triage",
			"The support classifier separates billing tickets from access tickets.",
		),
		(
			"analytics job watermark",
			"The analytics worker stores a warehouse watermark after each import.",
		),
	];

	TOPICS[index % TOPICS.len()]
}

pub(super) fn concurrency_probe_indexes(note_count: usize) -> Vec<usize> {
	let mut indexes = vec![0, note_count / 2, note_count.saturating_sub(1)];

	indexes.sort_unstable();
	indexes.dedup();

	indexes
}

pub(super) fn cost_proxy_report(
	notes: &[CorpusNote],
	queries: &[QueryResult],
	embedding: &EmbeddingRuntimeReport,
) -> CostProxyReport {
	reporting::cost_proxy_report_impl(notes, queries, embedding)
}

pub(super) fn latency_percentile(latencies: &[f64], percentile: f64) -> f64 {
	reporting::latency_percentile_impl(latencies, percentile)
}

pub(super) fn operational_cases() -> Vec<OperationalCase> {
	reporting::operational_cases_impl()
}

pub(super) fn incomplete_check(name: &'static str, reason: &str) -> CheckResult {
	reporting::incomplete_check_impl(name, reason)
}

pub(super) fn summarize_checks(checks: &[CheckResult]) -> CheckSummary {
	reporting::summarize_checks_impl(checks)
}

pub(super) fn project_status_from_summary(summary: &CheckSummary) -> &'static str {
	reporting::project_status_from_summary_impl(summary)
}

pub(super) async fn resource_envelope_check(
	service: &ElfService,
	corpus_dir: &Path,
	report_path: &Path,
	checkpoint_path: &Path,
	elapsed_seconds: f64,
) -> CheckResult {
	resource::resource_envelope_check_impl(
		service,
		corpus_dir,
		report_path,
		checkpoint_path,
		elapsed_seconds,
	)
	.await
}

pub(super) async fn run_lifecycle_checks(
	runtime: &BaselineRuntime,
	service: &ElfService,
	notes: &[CorpusNote],
	note_ids: &[Uuid],
) -> Result<Vec<CheckResult>> {
	lifecycle::run_lifecycle_checks_impl(runtime, service, notes, note_ids).await
}

pub(super) async fn run_concurrent_write_check(
	runtime: &BaselineRuntime,
	service: Arc<ElfService>,
) -> Result<CheckResult> {
	stress::run_concurrent_write_check_impl(runtime, service).await
}

pub(super) async fn run_soak_stability_check(
	runtime: &BaselineRuntime,
	service: Arc<ElfService>,
) -> Result<Option<CheckResult>> {
	stress::run_soak_stability_check_impl(runtime, service).await
}
