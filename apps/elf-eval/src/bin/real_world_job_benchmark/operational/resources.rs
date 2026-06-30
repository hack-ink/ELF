use crate::{
	BTreeSet, JobReport, OperationalColdStartRestoreRebuild, OperationalResourceSummary,
	RealWorldJob, TypedStatus, operational::tags,
};

pub(in crate::operational) fn operational_resource_summary(
	paired: &[(&RealWorldJob, &JobReport)],
) -> OperationalResourceSummary {
	let resource_jobs = paired
		.iter()
		.filter(|(job, _)| tags::job_has_tag(job, "resource_envelope"))
		.collect::<Vec<_>>();
	let latency_resource_dimension_job_count = paired
		.iter()
		.filter(|(_, report)| {
			report.dimension_scores.iter().any(|score| score.dimension == "latency_resource")
		})
		.count();

	OperationalResourceSummary {
		resource_envelope_job_count: resource_jobs.len(),
		resource_envelope_pass_count: resource_jobs
			.iter()
			.filter(|(_, report)| report.status == TypedStatus::Pass)
			.count(),
		latency_resource_dimension_job_count,
		job_ids: resource_jobs.iter().map(|(_, report)| report.job_id.clone()).collect(),
	}
}

pub(in crate::operational) fn operational_cold_start_restore_rebuild(
	paired: &[(&RealWorldJob, &JobReport)],
) -> OperationalColdStartRestoreRebuild {
	let cold_start_jobs =
		paired.iter().filter(|(job, _)| tags::job_has_tag(job, "cold_start")).collect::<Vec<_>>();
	let restore_jobs =
		paired.iter().filter(|(job, _)| tags::job_has_tag(job, "restore")).collect::<Vec<_>>();
	let qdrant_rebuild_jobs = paired
		.iter()
		.filter(|(job, report)| {
			tags::job_has_tag(job, "qdrant_rebuild") || report.qdrant_rebuild_case
		})
		.collect::<Vec<_>>();
	let mut job_ids = cold_start_jobs
		.iter()
		.chain(restore_jobs.iter())
		.chain(qdrant_rebuild_jobs.iter())
		.map(|(_, report)| report.job_id.clone())
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();

	job_ids.sort();
	OperationalColdStartRestoreRebuild {
		cold_start_job_count: cold_start_jobs.len(),
		cold_start_pass_count: cold_start_jobs
			.iter()
			.filter(|(_, report)| report.status == TypedStatus::Pass)
			.count(),
		restore_job_count: restore_jobs.len(),
		restore_pass_count: restore_jobs
			.iter()
			.filter(|(_, report)| report.status == TypedStatus::Pass)
			.count(),
		qdrant_rebuild_job_count: qdrant_rebuild_jobs.len(),
		qdrant_rebuild_pass_count: qdrant_rebuild_jobs
			.iter()
			.filter(|(_, report)| report.status == TypedStatus::Pass)
			.count(),
		job_ids,
	}
}
