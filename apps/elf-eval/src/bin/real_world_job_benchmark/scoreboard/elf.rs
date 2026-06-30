mod lifecycle;
mod metrics;
mod narrative;
mod operations;
mod retrieval;

use crate::scoreboard::{
	JobReport, RealWorldJob, ReportSummary, ScoreboardRow, TypedStatus, common,
};

pub(super) fn elf_scoreboard_row(
	raw_jobs: &[RealWorldJob],
	job_reports: &[JobReport],
	summary: &ReportSummary,
) -> ScoreboardRow {
	let source_id_mapped =
		summary.source_ref_required_count > 0 && summary.source_ref_coverage >= 1.0;
	let result_state = common::aggregate_job_report_state(job_reports);
	let metrics = metrics::scoreboard_metrics_for_reports(raw_jobs, job_reports, summary);
	let typed_non_pass_count =
		job_reports.iter().filter(|job| job.status != TypedStatus::Pass).count();
	let mut row = ScoreboardRow {
		product_id: "elf_current_report".to_string(),
		product_name: "ELF".to_string(),
		row_source: "current_real_world_job_report".to_string(),
		evidence_class: "fixture_backed".to_string(),
		result_state,
		comparable: false,
		same_corpus: true,
		source_id_mapped,
		held_out: common::jobs_have_tag(raw_jobs, "held_out"),
		leakage_audited: common::jobs_have_tag(raw_jobs, "leakage_audited"),
		product_runtime: false,
		container_digest_identified: false,
		metrics,
		strengths: narrative::elf_scoreboard_strengths(summary),
		weaknesses: Vec::new(),
		next_evidence: Vec::new(),
		source_provenance: vec![
			"apps/elf-eval/fixtures/real_world_memory/".to_string(),
			"apps/elf-eval/src/bin/real_world_job_benchmark/main.rs".to_string(),
		],
	};

	if typed_non_pass_count > 0 {
		row.weaknesses
			.push(format!("{typed_non_pass_count} encoded job row(s) are typed non-pass."));
	}

	common::scoreboard_apply_comparability_gaps(&mut row);

	row
}
