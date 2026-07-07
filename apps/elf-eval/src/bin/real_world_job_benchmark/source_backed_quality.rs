mod reports;

pub(super) use reports::{
	SourceBackedContextPackDecisionCounts, SourceBackedQualityMetrics, SourceBackedQualityReport,
	SourceBackedScenarioCoverage,
};

use crate::{
	BTreeSet, JobReport, RealWorldJob, ReportSummary, ScoreboardReport, TypedStatus, formatting,
};

const SOURCE_BACKED_QUALITY_SCHEMA: &str = "elf.source_backed_memory_quality_benchmark/v1";
const REQUIRED_METRICS: &[&str] = &[
	"expected_evidence_recall",
	"precision_at_5",
	"irrelevant_context_ratio",
	"source_ref_coverage",
	"stale_suppression_rate",
	"correction_persistence_rate",
	"delete_tombstone_suppression_rate",
	"unsupported_claim_rate",
	"cross_scope_leak_count",
	"journal_only_authority_claim_count",
	"context_pack_activation_precision",
	"context_pack_activation_recall",
	"activation_trace_coverage",
	"mean_latency_ms",
	"total_cost",
];
const REQUIRED_SCENARIOS: &[(&str, &[&str])] = &[
	("correct_source_backed_recall", &["source_backed_recall", "source_library"]),
	("source_backed_memory_promotion", &["memory_candidate", "approved_memory"]),
	("stale_memory_suppression", &["stale_fact", "stale_suppression"]),
	("correction_persistence", &["correction_persistence"]),
	("superseded_memory_not_current", &["superseded", "archival_supersession"]),
	("delete_tombstone_suppression", &["delete", "tombstone"]),
	("cross_project_private_scope_leak_trap", &["scope_leak_trap", "privacy_leak"]),
	("read_profile_downgrade_trap", &["read_profile_downgrade"]),
	("pinned_pack_trap", &["pinned_pack_trap"]),
	("journal_only_current_fact_trap", &["journal_only_authority_trap", "janitor"]),
	("where_stopped_resume", &["where_stopped", "reset_resume"]),
	("dreaming_no_silent_mutation", &["dreaming_no_silent_mutation", "consolidation"]),
	("context_pack_relevant_auto_activation", &["context_pack_relevant_activation"]),
	("context_pack_irrelevant_suppression", &["context_pack_irrelevant_suppression"]),
	("context_pack_disabled_suppression", &["context_pack_disabled_suppression"]),
	("context_pack_stale_suppression", &["context_pack_stale_suppression"]),
	(
		"derived_knowledge_stale_on_source_change",
		&["changed_source_watch_rebuild", "watch_rebuild"],
	),
	("authoritative_revalidation", &["authoritative_revalidation", "qdrant_rebuild"]),
	("recall_debug_reason_codes", &["recall_debug_reason_codes", "operator_debug"]),
	("recall_debug_privacy", &["recall_debug_privacy", "redaction"]),
];

pub(super) fn source_backed_quality_report(
	raw_jobs: &[RealWorldJob],
	job_reports: &[JobReport],
	summary: &ReportSummary,
	scoreboard: &ScoreboardReport,
) -> SourceBackedQualityReport {
	let first_elf_row =
		scoreboard.rows.iter().find(|row| row.product_id == "elf" || row.product_name == "ELF");
	let lifecycle = first_elf_row.map(|row| &row.metrics.lifecycle);
	let retrieval = first_elf_row.map(|row| &row.metrics.retrieval);
	let work = summary.work_continuity.as_ref();
	let correction_jobs = tagged_job_reports(raw_jobs, job_reports, |job| {
		has_any_tag(job, &["correction_persistence", "correction"])
	});
	let delete_jobs = raw_jobs
		.iter()
		.zip(job_reports.iter())
		.filter(|(job, _)| {
			has_any_tag(job, &["delete", "tombstone"])
				|| job
					.memory_evolution
					.as_ref()
					.is_some_and(|evolution| !evolution.tombstone_evidence_ids.is_empty())
		})
		.map(|(_, report)| report)
		.collect::<Vec<_>>();
	let scenario_coverage = REQUIRED_SCENARIOS
		.iter()
		.map(|(scenario, tags)| scenario_coverage(raw_jobs, job_reports, scenario, tags))
		.collect::<Vec<_>>();
	let context_pack_decisions = context_pack_decision_counts(raw_jobs);
	let typed_result_states_present = typed_result_states_present(job_reports);
	let cross_scope_leak_count = summary.scope_violation_count + summary.redaction_leak_count;
	let journal_only_authority_claim_count =
		work.map_or(0, |work| work.journal_only_authority_claim_count);
	let mut hard_failures = Vec::new();

	if cross_scope_leak_count > 0 {
		hard_failures.push("cross_scope_leak_count_nonzero".to_string());
	}
	if journal_only_authority_claim_count > 0 {
		hard_failures.push("journal_only_authority_claim_count_nonzero".to_string());
	}
	if context_pack_decisions.total_decisions == 0 {
		hard_failures.push("context_pack_decisions_not_encoded".to_string());
	}
	if context_pack_decisions.incorrect_decisions > 0 {
		hard_failures.push(format!(
			"context_pack_incorrect_decision_count:{}",
			context_pack_decisions.incorrect_decisions
		));
	}

	for report in required_job_reports(raw_jobs, job_reports) {
		if !report.hard_fail_hits.is_empty() {
			hard_failures.push(format!("required_job_hard_fail:{}", report.job_id));
		}
		if !report.trap_ids_used.is_empty() {
			hard_failures.push(format!("required_job_trap_used:{}", report.job_id));
		}
	}
	for scenario in &scenario_coverage {
		if scenario.status != "pass" {
			hard_failures.push(format!(
				"required_scenario_non_pass:{}:{}",
				scenario.scenario, scenario.status
			));
		}
	}

	SourceBackedQualityReport {
		schema: SOURCE_BACKED_QUALITY_SCHEMA.to_string(),
		metric_basis: "real_world_job_benchmark_fixture_and_product_runtime_rows".to_string(),
		result_state: if hard_failures.is_empty() { "pass" } else { "not_encoded" }.to_string(),
		hard_fail_passed: hard_failures.is_empty(),
		hard_failures,
		required_metric_names: REQUIRED_METRICS
			.iter()
			.map(|metric| (*metric).to_string())
			.collect(),
		metrics: SourceBackedQualityMetrics {
			expected_evidence_recall: summary.expected_evidence_recall,
			precision_at_5: retrieval.and_then(|retrieval| retrieval.precision_at_k),
			irrelevant_context_ratio: summary.irrelevant_context_ratio,
			source_ref_coverage: summary.source_ref_coverage,
			stale_suppression_rate: lifecycle.and_then(|lifecycle| lifecycle.stale_suppression),
			correction_persistence_rate: pass_rate(correction_jobs.as_slice()),
			delete_tombstone_suppression_rate: pass_rate(delete_jobs.as_slice()),
			unsupported_claim_rate: formatting::round3(
				summary.unsupported_claim_count as f64 / summary.job_count.max(1) as f64,
			),
			cross_scope_leak_count,
			journal_only_authority_claim_count,
			context_pack_activation_precision: context_pack_activation_precision(
				&context_pack_decisions,
			),
			context_pack_activation_recall: context_pack_activation_recall(&context_pack_decisions),
			activation_trace_coverage: context_pack_trace_coverage(&context_pack_decisions),
			mean_latency_ms: summary.mean_latency_ms,
			total_cost: summary.total_cost.clone(),
		},
		context_pack_decisions,
		scenario_coverage,
		typed_result_states_present,
		artifact_policy: concat!(
			"pass means executable fixture/product-runtime evidence exists; wrong_result, ",
			"incomplete, blocked, not_tested, not_encoded, and unsupported_claim remain typed ",
			"non-pass evidence and must not be collapsed into wins."
		)
		.to_string(),
	}
}

pub(super) fn validate_source_backed_quality_gate(
	report: &SourceBackedQualityReport,
) -> Result<(), Vec<String>> {
	let mut failures = Vec::new();

	if report.schema != SOURCE_BACKED_QUALITY_SCHEMA {
		failures.push(format!("unexpected_schema:{}", report.schema));
	}
	if report.result_state != "pass" {
		failures.push(format!("result_state_non_pass:{}", report.result_state));
	}
	if !report.hard_fail_passed {
		failures.push("hard_fail_passed_false".to_string());
	}
	if !report.hard_failures.is_empty() {
		failures
			.extend(report.hard_failures.iter().map(|failure| format!("hard_failure:{failure}")));
	}

	for metric in REQUIRED_METRICS {
		if !report.required_metric_names.iter().any(|name| name == metric) {
			failures.push(format!("missing_required_metric:{metric}"));
		}
	}
	for scenario in &report.scenario_coverage {
		if scenario.required && scenario.status != "pass" {
			failures.push(format!(
				"required_scenario_non_pass:{}:{}",
				scenario.scenario, scenario.status
			));
		}
	}

	if report.context_pack_decisions.total_decisions == 0 {
		failures.push("context_pack_decisions_not_encoded".to_string());
	}
	if report.context_pack_decisions.incorrect_decisions > 0 {
		failures.push(format!(
			"context_pack_incorrect_decision_count:{}",
			report.context_pack_decisions.incorrect_decisions
		));
	}

	if failures.is_empty() { Ok(()) } else { Err(failures) }
}

fn scenario_coverage(
	raw_jobs: &[RealWorldJob],
	job_reports: &[JobReport],
	scenario: &str,
	tags: &[&str],
) -> SourceBackedScenarioCoverage {
	let reports = raw_jobs
		.iter()
		.zip(job_reports.iter())
		.filter(|(job, _)| has_any_tag(job, tags))
		.map(|(_, report)| report)
		.collect::<Vec<_>>();
	let pass_count = reports.iter().filter(|report| report.status == TypedStatus::Pass).count();
	let status = if reports.is_empty() {
		"not_encoded"
	} else if pass_count == reports.len() {
		"pass"
	} else if reports.iter().any(|report| report.status == TypedStatus::WrongResult) {
		"wrong_result"
	} else if reports.iter().any(|report| report.status == TypedStatus::Blocked) {
		"blocked"
	} else if reports.iter().any(|report| report.status == TypedStatus::Incomplete) {
		"incomplete"
	} else if reports.iter().any(|report| report.status == TypedStatus::UnsupportedClaim) {
		"unsupported_claim"
	} else {
		"not_encoded"
	};

	SourceBackedScenarioCoverage {
		scenario: scenario.to_string(),
		status: status.to_string(),
		covered_job_count: reports.len(),
		pass_count,
		required: true,
	}
}

fn tagged_job_reports<'a>(
	raw_jobs: &'a [RealWorldJob],
	job_reports: &'a [JobReport],
	predicate: impl Fn(&RealWorldJob) -> bool,
) -> Vec<&'a JobReport> {
	raw_jobs
		.iter()
		.zip(job_reports.iter())
		.filter(|(job, _)| predicate(job))
		.map(|(_, report)| report)
		.collect()
}

fn has_any_tag(job: &RealWorldJob, tags: &[&str]) -> bool {
	job.tags.iter().any(|tag| tags.iter().any(|expected| tag == expected))
}

fn pass_rate(reports: &[&JobReport]) -> Option<f64> {
	if reports.is_empty() {
		None
	} else {
		let pass_count = reports.iter().filter(|report| report.status == TypedStatus::Pass).count();

		Some(formatting::round3(pass_count as f64 / reports.len() as f64))
	}
}

fn required_job_reports<'a>(
	raw_jobs: &'a [RealWorldJob],
	job_reports: &'a [JobReport],
) -> Vec<&'a JobReport> {
	raw_jobs
		.iter()
		.zip(job_reports.iter())
		.filter(|(job, _)| {
			REQUIRED_SCENARIOS
				.iter()
				.any(|(_, tags)| tags.iter().any(|tag| has_any_tag(job, &[*tag])))
		})
		.map(|(_, report)| report)
		.collect()
}

fn context_pack_decision_counts(
	raw_jobs: &[RealWorldJob],
) -> SourceBackedContextPackDecisionCounts {
	let mut counts = SourceBackedContextPackDecisionCounts::default();

	for job in raw_jobs.iter().filter(|job| {
		job.tags.iter().any(|tag| tag.starts_with("context_pack_")) || job.context_pack.is_some()
	}) {
		let Some(context_pack) = job.context_pack.as_ref() else {
			continue;
		};

		for decision in &context_pack.decisions {
			let correct = decision.expected_state == decision.observed_state;

			counts.total_decisions += 1;

			if decision.decision_id.trim().is_empty() || decision.layer.trim().is_empty() {
				counts.incorrect_decisions += 1;
			}
			if !correct {
				counts.incorrect_decisions += 1;
			}
			if !decision.reason_code.trim().is_empty() && !decision.source_refs.is_empty() {
				counts.traced_decisions += 1;
			}

			match decision.expected_state.as_str() {
				"enabled" => {
					counts.expected_enabled += 1;

					if correct {
						counts.correct_enabled += 1;
					}
				},
				"suppressed" => {
					counts.expected_suppressed += 1;

					if correct {
						counts.correct_suppressed += 1;
					}
				},
				"disabled" => {
					counts.expected_disabled += 1;

					if correct {
						counts.correct_disabled += 1;
					}
				},
				"stale_suppressed" => {
					counts.expected_stale_suppressed += 1;

					if correct {
						counts.correct_stale_suppressed += 1;
					}
				},
				"blocked" => {
					counts.expected_blocked += 1;

					if correct {
						counts.correct_blocked += 1;
					}
				},
				"pinned_ineligible" => {
					counts.expected_pinned_ineligible += 1;

					if correct && decision.pinned {
						counts.correct_pinned_ineligible += 1;
					}
				},
				_ => counts.incorrect_decisions += 1,
			}

			if decision.observed_state == "enabled" {
				counts.observed_enabled += 1;
			}
		}
	}

	counts
}

fn context_pack_activation_precision(
	counts: &SourceBackedContextPackDecisionCounts,
) -> Option<f64> {
	if counts.observed_enabled == 0 {
		None
	} else {
		Some(formatting::round3(counts.correct_enabled as f64 / counts.observed_enabled as f64))
	}
}

fn context_pack_activation_recall(counts: &SourceBackedContextPackDecisionCounts) -> Option<f64> {
	if counts.expected_enabled == 0 {
		None
	} else {
		Some(formatting::round3(counts.correct_enabled as f64 / counts.expected_enabled as f64))
	}
}

fn context_pack_trace_coverage(counts: &SourceBackedContextPackDecisionCounts) -> Option<f64> {
	if counts.total_decisions == 0 {
		None
	} else {
		Some(formatting::round3(counts.traced_decisions as f64 / counts.total_decisions as f64))
	}
}

fn typed_result_states_present(job_reports: &[JobReport]) -> Vec<String> {
	job_reports
		.iter()
		.map(|report| match report.status {
			TypedStatus::Pass => "pass",
			TypedStatus::WrongResult => "wrong_result",
			TypedStatus::LifecycleFail => "lifecycle_fail",
			TypedStatus::Incomplete => "incomplete",
			TypedStatus::Blocked => "blocked",
			TypedStatus::NotEncoded => "not_encoded",
			TypedStatus::UnsupportedClaim => "unsupported_claim",
		})
		.map(str::to_string)
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect()
}
