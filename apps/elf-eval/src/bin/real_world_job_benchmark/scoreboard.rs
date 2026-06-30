mod common;
mod elf;
mod external;

use crate::{
	AdapterCoverageStatus, AdapterStatusCounts, BTreeMap, BTreeSet, ExternalAdapterReport,
	ExternalAdapterSection, ExternalAdapterSummary, JobReport, RealWorldJob, ReportSummary,
	SCOREBOARD_EVIDENCE_CLASSES, SCOREBOARD_RESULT_STATES, SCOREBOARD_RETRIEVAL_K,
	SCOREBOARD_SCHEMA, ScenarioComparisonOutcome, ScoreboardAnswerSafetyMetrics,
	ScoreboardCoverageMetrics, ScoreboardLifecycleMetrics, ScoreboardMetrics,
	ScoreboardOperationalMetrics, ScoreboardRankedMetrics, ScoreboardReport,
	ScoreboardRetrievalMetrics, ScoreboardRow, TypedStatus,
	formatting::{adapter_status_str, round3},
	scenario_comparison_outcome,
	summary::{aggregate_status, ratio, ratio_or},
};

pub(super) fn scoreboard_report(
	raw_jobs: &[RealWorldJob],
	job_reports: &[JobReport],
	summary: &ReportSummary,
	external_adapters: &ExternalAdapterSection,
) -> ScoreboardReport {
	let job_typed_non_pass_count =
		job_reports.iter().filter(|job| job.status != TypedStatus::Pass).count();
	let external_typed_non_pass_count =
		common::external_typed_non_pass_count(&external_adapters.summary);
	let job_typed_non_pass_states_present = common::typed_non_pass_states_present(job_reports);
	let external_adapter_typed_non_pass_states_present =
		common::external_typed_non_pass_states_present(&external_adapters.summary);
	let mut typed_non_pass_states_present = job_typed_non_pass_states_present.clone();

	typed_non_pass_states_present.extend(external_adapter_typed_non_pass_states_present.clone());
	typed_non_pass_states_present.sort();
	typed_non_pass_states_present.dedup();

	let typed_non_pass_count = job_typed_non_pass_count + external_typed_non_pass_count;

	ScoreboardReport {
		schema: SCOREBOARD_SCHEMA.to_string(),
		result_states: SCOREBOARD_RESULT_STATES.iter().map(ToString::to_string).collect(),
		evidence_classes: SCOREBOARD_EVIDENCE_CLASSES.iter().map(ToString::to_string).collect(),
		metric_basis: "produced_evidence_order".to_string(),
		retrieval_k: SCOREBOARD_RETRIEVAL_K,
		job_typed_non_pass_count,
		job_typed_non_pass_states_present,
		job_summary_claim: common::scoreboard_summary_claim(job_reports, job_typed_non_pass_count)
			.to_string(),
		external_adapter_typed_non_pass_count: external_typed_non_pass_count,
		external_adapter_typed_non_pass_states_present,
		typed_non_pass_count,
		typed_non_pass_states_present,
		evidence_class_counts: common::scoreboard_evidence_class_counts(external_adapters),
		summary_claim: common::scoreboard_summary_claim(job_reports, typed_non_pass_count)
			.to_string(),
		unqualified_win_claim_allowed: false,
		claim_boundary: "Typed non-pass states and non-live evidence classes must remain visible; reports must not collapse them into unqualified wins.".to_string(),
		rows: scoreboard_rows(raw_jobs, job_reports, summary, external_adapters),
		optimization_roadmap: common::scoreboard_optimization_roadmap(),
	}
}

fn scoreboard_rows(
	raw_jobs: &[RealWorldJob],
	job_reports: &[JobReport],
	summary: &ReportSummary,
	external_adapters: &ExternalAdapterSection,
) -> Vec<ScoreboardRow> {
	let mut rows = vec![elf::elf_scoreboard_row(raw_jobs, job_reports, summary)];

	rows.extend(external::external_project_scoreboard_rows(&external_adapters.adapters));

	rows
}
