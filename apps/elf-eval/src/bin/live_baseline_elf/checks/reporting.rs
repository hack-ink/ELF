use crate::checks::{
	CheckResult, CheckSummary, CorpusNote, CostProxyReport, EmbeddingRuntimeReport,
	OperationalCase, QueryResult, env,
};

pub(super) fn cost_proxy_report_impl(
	notes: &[CorpusNote],
	queries: &[QueryResult],
	embedding: &EmbeddingRuntimeReport,
) -> CostProxyReport {
	let note_chars = notes.iter().map(|note| note.text.len()).sum::<usize>();
	let query_chars = queries.iter().map(|query| query.query.len()).sum::<usize>();
	let estimated_input_chars = note_chars.saturating_add(query_chars);
	let estimated_input_tokens = estimated_input_chars.saturating_add(3) / 4;
	let configured_usd_per_1k_tokens = env::var("ELF_BASELINE_COST_PER_1K_TOKENS_USD")
		.ok()
		.and_then(|value| value.parse::<f64>().ok());
	let estimated_usd =
		configured_usd_per_1k_tokens.map(|rate| estimated_input_tokens as f64 / 1_000.0 * rate);

	CostProxyReport {
		schema: "elf.live_baseline.cost_proxy/v1",
		scope: "primary corpus note text plus declared same-corpus query text",
		embedding_mode: embedding.mode,
		estimated_input_chars,
		estimated_input_tokens,
		token_estimation: "ceil(ascii_utf8_chars / 4)",
		configured_usd_per_1k_tokens,
		estimated_usd,
		document_count: notes.len(),
		query_count: queries.len(),
	}
}

pub(super) fn latency_percentile_impl(latencies: &[f64], percentile: f64) -> f64 {
	if latencies.is_empty() {
		return 0.0;
	}

	let mut sorted = latencies.to_vec();

	sorted.sort_by(f64::total_cmp);

	let rank = ((sorted.len().saturating_sub(1)) as f64 * percentile).ceil() as usize;

	sorted[rank.min(sorted.len().saturating_sub(1))]
}

pub(super) fn operational_cases_impl() -> Vec<OperationalCase> {
	vec![
		operational_case(
			"private_corpus_addendum",
			"fails_closed_without_manifest",
			"opt_in",
			"ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST=tmp/private-production-corpus/manifest.json cargo make baseline-production-private-addendum",
			"tmp/live-baseline/private-production-addendum.md",
			"Markdown addendum reports manifest id, evidence ids, tasks, checks, latency, resource, and cost proxy fields; private text remains in tmp JSON/logs only.",
		),
		operational_case(
			"backfill_10k_resume",
			"not_run",
			"opt_in",
			"cargo make baseline-backfill-10k-docker",
			"tmp/live-baseline/live-baseline-report.json",
			"Runs Docker-owned dependencies and records checkpoint resume, duplicates, latency percentiles, resource usage, and cost proxy fields.",
		),
		operational_case(
			"backfill_100k_resume",
			"guarded",
			"expensive_opt_in",
			"ELF_BASELINE_ENABLE_EXPENSIVE=1 cargo make baseline-backfill-100k-docker",
			"tmp/live-baseline/live-baseline-report.json",
			"Fails closed unless the expensive-run guard is explicitly enabled.",
		),
		operational_case(
			"provider_outage",
			"not_run",
			"documented_operator_probe",
			"ELF_BASELINE_ELF_EMBEDDING_MODE=provider with an unavailable embedding endpoint and cargo make baseline-production-synthetic",
			"ELF project status incomplete or blocked with provider failure in tmp/live-baseline/ELF.log",
			"Use only synthetic or sanitized manifests; do not place provider keys in committed files.",
		),
		operational_case(
			"compose_start_stop_upgrade",
			"documented",
			"runbook",
			"docs/runbook/single_user_production.md Sections 2, 4, and 5",
			"storage health, API health, migration check, and post-upgrade search smoke",
			"Backup Postgres before binary/config upgrade; rollback restores the previous backup and rebuilds Qdrant.",
		),
		operational_case(
			"postgres_restore_qdrant_rebuild",
			"documented",
			"runbook_or_clean_volume_proof",
			"docs/runbook/single_user_production.md Sections 6 through 9",
			"Postgres restored row count, admin qdrant rebuild counts, and search-after-restore response",
			"Qdrant remains derived and rebuild uses Postgres-held vectors without embedding provider calls.",
		),
		operational_case(
			"migration_rollback",
			"documented",
			"runbook",
			"docs/runbook/single_user_production.md Section 5 rollback path",
			"pre-upgrade backup path, restored source rows, qdrant rebuild, and health check",
			"No reverse migration is claimed; rollback means previous binary/config plus restored Postgres backup.",
		),
		operational_case(
			"unattended_soak",
			"bounded",
			"opt_in",
			"ELF_BASELINE_PROJECTS=ELF ELF_BASELINE_PROFILE=stress ELF_BASELINE_SOAK_SECONDS=3600 cargo make baseline-live-docker",
			"soak_stability_e2e check and resource_envelope check in tmp/live-baseline/live-baseline-report.json",
			"Long soak duration is env-controlled and not part of the default smoke profile.",
		),
	]
}

pub(super) fn incomplete_check_impl(name: &'static str, reason: &str) -> CheckResult {
	CheckResult {
		name,
		status: "incomplete",
		reason: reason.to_string(),
		evidence: serde_json::json!({}),
	}
}

pub(super) fn summarize_checks_impl(checks: &[CheckResult]) -> CheckSummary {
	let wrong_result = checks.iter().filter(|check| check.status == "wrong_result").count();
	let lifecycle_fail = checks.iter().filter(|check| check.status == "lifecycle_fail").count();

	CheckSummary {
		total: checks.len(),
		pass: checks.iter().filter(|check| check.status == "pass").count(),
		fail: wrong_result + lifecycle_fail,
		wrong_result,
		lifecycle_fail,
		incomplete: checks.iter().filter(|check| check.status == "incomplete").count(),
		blocked: checks.iter().filter(|check| check.status == "blocked").count(),
		not_encoded: checks.iter().filter(|check| check.status == "not_encoded").count(),
	}
}

pub(super) fn project_status_from_summary_impl(summary: &CheckSummary) -> &'static str {
	if summary.wrong_result > 0 {
		"wrong_result"
	} else if summary.lifecycle_fail > 0 {
		"lifecycle_fail"
	} else if summary.blocked > 0 {
		"blocked"
	} else if summary.incomplete > 0 {
		"incomplete"
	} else if summary.not_encoded > 0 {
		"not_encoded"
	} else {
		"pass"
	}
}

fn operational_case(
	name: &'static str,
	default_status: &'static str,
	operator_status: &'static str,
	command: &'static str,
	evidence: &'static str,
	safety: &'static str,
) -> OperationalCase {
	OperationalCase { name, default_status, operator_status, command, evidence, safety }
}
