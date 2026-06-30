use crate::{
	JobReport, OperationalEvidenceTierReport, RealWorldJob, TypedStatus,
	operational::{self, tags},
	summary,
};

pub(in crate::operational) fn operational_evidence_tier_report(
	tier: &str,
	paired: &[(&RealWorldJob, &JobReport)],
) -> OperationalEvidenceTierReport {
	let tier_jobs = paired
		.iter()
		.filter(|(job, _)| operational::operational_evidence_tier(job) == tier)
		.copied()
		.collect::<Vec<_>>();
	let reports = tier_jobs.iter().map(|(_, report)| *report).collect::<Vec<_>>();
	let status = if reports.is_empty() {
		TypedStatus::NotEncoded
	} else {
		summary::aggregate_status(reports.as_slice())
	};
	let job_count = reports.len();
	let pass = reports.iter().filter(|report| report.status == TypedStatus::Pass).count();
	let wrong_result =
		reports.iter().filter(|report| report.status == TypedStatus::WrongResult).count();
	let lifecycle_fail =
		reports.iter().filter(|report| report.status == TypedStatus::LifecycleFail).count();
	let incomplete =
		reports.iter().filter(|report| report.status == TypedStatus::Incomplete).count();
	let blocked = reports.iter().filter(|report| report.status == TypedStatus::Blocked).count();
	let not_encoded = usize::from(reports.is_empty())
		+ reports.iter().filter(|report| report.status == TypedStatus::NotEncoded).count();
	let unsupported_claim =
		reports.iter().filter(|report| report.status == TypedStatus::UnsupportedClaim).count();

	OperationalEvidenceTierReport {
		tier: tier.to_string(),
		status,
		job_count,
		pass,
		wrong_result,
		lifecycle_fail,
		incomplete,
		blocked,
		not_encoded,
		unsupported_claim,
		mean_latency_ms: summary::mean_latency_for_reports(reports.as_slice()),
		total_cost: summary::total_cost_for_reports(reports.as_slice()),
		resource_evidence_count: tier_jobs
			.iter()
			.filter(|(job, _)| tags::job_has_tag(job, "resource_envelope"))
			.count(),
		cold_start_evidence_count: tier_jobs
			.iter()
			.filter(|(job, _)| tags::job_has_tag(job, "cold_start"))
			.count(),
		restore_evidence_count: tier_jobs
			.iter()
			.filter(|(job, _)| tags::job_has_tag(job, "restore"))
			.count(),
		qdrant_rebuild_evidence_count: tier_jobs
			.iter()
			.filter(|(job, report)| {
				tags::job_has_tag(job, "qdrant_rebuild") || report.qdrant_rebuild_case
			})
			.count(),
		pass_claim_allowed: job_count > 0 && status == TypedStatus::Pass,
		blocker_reasons: reports
			.iter()
			.filter(|report| report.status != TypedStatus::Pass)
			.map(|report| report.reason.clone())
			.collect(),
		job_ids: reports.iter().map(|report| report.job_id.clone()).collect(),
	}
}

pub(in crate::operational) fn operational_tier_has_typed_blocker(
	tier: &OperationalEvidenceTierReport,
) -> bool {
	tier.blocked + tier.incomplete + tier.not_encoded > 0 && !tier.pass_claim_allowed
}
