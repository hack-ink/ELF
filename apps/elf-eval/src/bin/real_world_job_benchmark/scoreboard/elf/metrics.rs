use crate::scoreboard::{
	self, JobReport, RealWorldJob, ReportSummary, ScoreboardAnswerSafetyMetrics,
	ScoreboardCoverageMetrics, ScoreboardMetrics, TypedStatus,
	elf::{lifecycle, operations, retrieval},
};

pub(in crate::scoreboard::elf) fn scoreboard_metrics_for_reports(
	raw_jobs: &[RealWorldJob],
	job_reports: &[JobReport],
	summary: &ReportSummary,
) -> ScoreboardMetrics {
	ScoreboardMetrics {
		retrieval: retrieval::scoreboard_retrieval_metrics(job_reports, summary),
		lifecycle: lifecycle::scoreboard_lifecycle_metrics(raw_jobs, job_reports),
		answer_safety: scoreboard_answer_safety_metrics(summary),
		operations: operations::scoreboard_operational_metrics(raw_jobs, job_reports, summary),
		coverage: ScoreboardCoverageMetrics {
			job_count: summary.job_count,
			encoded_suite_count: summary.encoded_suite_count,
			pass_count: summary.pass,
			typed_non_pass_count: job_reports
				.iter()
				.filter(|job| job.status != TypedStatus::Pass)
				.count(),
			source_ref_coverage: Some(summary.source_ref_coverage),
			evidence_coverage: Some(summary.evidence_coverage),
			evidence_class: "fixture_backed".to_string(),
		},
	}
}

fn scoreboard_answer_safety_metrics(summary: &ReportSummary) -> ScoreboardAnswerSafetyMetrics {
	ScoreboardAnswerSafetyMetrics {
		unsupported_claim_rate: Some(scoreboard::ratio(
			summary.unsupported_claim_count,
			summary.job_count,
		)),
		unsupported_claim_count: summary.unsupported_claim_count,
		stale_answer_rate: Some(scoreboard::ratio(summary.stale_answer_count, summary.job_count)),
		stale_answer_count: summary.stale_answer_count,
		hallucinated_evidence_rate: Some(summary.irrelevant_context_ratio),
		redaction_leak_count: summary.redaction_leak_count,
		irrelevant_context_ratio: Some(summary.irrelevant_context_ratio),
	}
}
