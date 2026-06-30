use crate::scoreboard::{
	self, JobReport, RealWorldJob, ScoreboardLifecycleMetrics, TypedStatus, common,
};

pub(in crate::scoreboard::elf) fn scoreboard_lifecycle_metrics(
	raw_jobs: &[RealWorldJob],
	job_reports: &[JobReport],
) -> ScoreboardLifecycleMetrics {
	let stale_check_count: usize = raw_jobs
		.iter()
		.map(|job| {
			job.negative_traps
				.iter()
				.filter(|trap| trap.failure_if_used && trap.trap_type == "stale_fact")
				.count()
		})
		.sum();
	let stale_failure_count = job_reports
		.iter()
		.map(|job| job.stale_answer_count + job.stale_retrieval_count)
		.sum::<usize>();
	let update_check_count =
		scoreboard_lifecycle_check_count(raw_jobs, common::scoreboard_is_update_job);
	let update_correct_count =
		scoreboard_lifecycle_correct_count(raw_jobs, job_reports, common::scoreboard_is_update_job);
	let delete_check_count =
		scoreboard_lifecycle_check_count(raw_jobs, common::scoreboard_is_delete_job);
	let delete_correct_count =
		scoreboard_lifecycle_correct_count(raw_jobs, job_reports, common::scoreboard_is_delete_job);
	let rollback_history_check_count =
		scoreboard_lifecycle_check_count(raw_jobs, common::scoreboard_is_rollback_history_job);
	let rollback_history_readback_count = raw_jobs
		.iter()
		.zip(job_reports.iter())
		.filter(|(job, report)| {
			common::scoreboard_is_rollback_history_job(job) && report.status == TypedStatus::Pass
		})
		.count();

	ScoreboardLifecycleMetrics {
		stale_suppression: Some(scoreboard::ratio_or(
			stale_check_count.saturating_sub(stale_failure_count),
			stale_check_count,
			1.0,
		)),
		stale_suppressed_count: stale_check_count.saturating_sub(stale_failure_count),
		stale_check_count,
		update_correctness: Some(scoreboard::ratio_or(
			update_correct_count,
			update_check_count,
			1.0,
		)),
		update_correct_count,
		update_check_count,
		delete_correctness: Some(scoreboard::ratio_or(
			delete_correct_count,
			delete_check_count,
			1.0,
		)),
		delete_correct_count,
		delete_check_count,
		rollback_history_readback_rate: Some(scoreboard::ratio_or(
			rollback_history_readback_count,
			rollback_history_check_count,
			1.0,
		)),
		rollback_history_readback_count,
		rollback_history_check_count,
	}
}

fn scoreboard_lifecycle_check_count(
	jobs: &[RealWorldJob],
	predicate: fn(&RealWorldJob) -> bool,
) -> usize {
	jobs.iter().filter(|job| predicate(job)).count()
}

fn scoreboard_lifecycle_correct_count(
	raw_jobs: &[RealWorldJob],
	job_reports: &[JobReport],
	predicate: fn(&RealWorldJob) -> bool,
) -> usize {
	raw_jobs
		.iter()
		.zip(job_reports.iter())
		.filter(|(job, report)| predicate(job) && report.status == TypedStatus::Pass)
		.count()
}
