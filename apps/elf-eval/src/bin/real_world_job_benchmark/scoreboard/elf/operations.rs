use crate::scoreboard::{
	JobReport, RealWorldJob, ReportSummary, ScoreboardOperationalMetrics, TypedStatus, common,
};

pub(in crate::scoreboard::elf) fn scoreboard_operational_metrics(
	raw_jobs: &[RealWorldJob],
	job_reports: &[JobReport],
	summary: &ReportSummary,
) -> ScoreboardOperationalMetrics {
	let resource_envelope_job_count = raw_jobs
		.iter()
		.filter(|job| common::scoreboard_has_any_tag(job, &["resource_envelope"]))
		.count();
	let resource_envelope_pass_count = raw_jobs
		.iter()
		.zip(job_reports.iter())
		.filter(|(job, report)| {
			common::scoreboard_has_any_tag(job, &["resource_envelope"])
				&& report.status == TypedStatus::Pass
		})
		.count();

	ScoreboardOperationalMetrics {
		mean_latency_ms: summary.mean_latency_ms,
		total_cost: summary.total_cost.clone(),
		resource_envelope_status: if resource_envelope_job_count == resource_envelope_pass_count {
			"pass".to_string()
		} else {
			"typed_non_pass_present".to_string()
		},
		resource_envelope_job_count,
		resource_envelope_pass_count,
	}
}
