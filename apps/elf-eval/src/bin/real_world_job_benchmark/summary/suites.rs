use crate::{
	BTreeSet, JobReport, NOT_ENCODED_REASON, SUITES, SuiteReport, TypedStatus, formatting,
	summary::{self},
};

pub(super) fn suite_reports_impl(jobs: &[JobReport]) -> Vec<SuiteReport> {
	SUITES.iter().map(|suite_id| suite_report(suite_id, jobs)).collect()
}

pub(super) fn aggregate_status_impl(jobs: &[&JobReport]) -> TypedStatus {
	let statuses = jobs.iter().map(|job| job.status).collect::<BTreeSet<_>>();

	if statuses.contains(&TypedStatus::UnsupportedClaim) {
		TypedStatus::UnsupportedClaim
	} else if statuses.contains(&TypedStatus::LifecycleFail) {
		TypedStatus::LifecycleFail
	} else if statuses.contains(&TypedStatus::WrongResult) {
		TypedStatus::WrongResult
	} else if statuses.contains(&TypedStatus::Incomplete) {
		TypedStatus::Incomplete
	} else if statuses.contains(&TypedStatus::Blocked) {
		TypedStatus::Blocked
	} else if statuses.contains(&TypedStatus::NotEncoded) {
		TypedStatus::NotEncoded
	} else if statuses.contains(&TypedStatus::Pass) {
		TypedStatus::Pass
	} else {
		TypedStatus::NotEncoded
	}
}

fn suite_report(suite_id: &str, jobs: &[JobReport]) -> SuiteReport {
	let suite_jobs = jobs.iter().filter(|job| job.suite_id == suite_id).collect::<Vec<_>>();

	if suite_jobs.is_empty() {
		return SuiteReport {
			suite_id: suite_id.to_string(),
			status: TypedStatus::NotEncoded,
			encoded_job_count: 0,
			score_mean: None,
			unsupported_claim_count: 0,
			wrong_result_count: 0,
			stale_answer_count: 0,
			conflict_detection_count: 0,
			update_rationale_available_count: 0,
			temporal_validity_not_encoded_count: 0,
			history_readback_encoded_count: 0,
			expected_evidence_recall: None,
			irrelevant_context_ratio: None,
			trace_explainability_count: 0,
			reason: NOT_ENCODED_REASON.to_string(),
		};
	}

	let status = aggregate_status_impl(&suite_jobs);
	let score_sum = suite_jobs.iter().map(|job| job.normalized_score).sum::<f64>();
	let unsupported_claim_count = suite_jobs.iter().map(|job| job.unsupported_claim_count).sum();
	let wrong_result_count = suite_jobs.iter().map(|job| job.wrong_result_count).sum();
	let stale_answer_count = suite_jobs.iter().map(|job| job.stale_answer_count).sum();
	let conflict_detection_count = suite_jobs.iter().map(|job| job.conflict_detection_count).sum();
	let update_rationale_available_count =
		suite_jobs.iter().filter(|job| job.update_rationale_available).count();
	let temporal_validity_not_encoded_count =
		suite_jobs.iter().filter(|job| job.temporal_validity_not_encoded).count();
	let history_readback_encoded_count =
		suite_jobs.iter().filter(|job| job.history_readback_encoded).count();
	let trace_explainability_count =
		suite_jobs.iter().filter(|job| job.trace_explainability.is_some()).count();

	SuiteReport {
		suite_id: suite_id.to_string(),
		status,
		encoded_job_count: suite_jobs.len(),
		score_mean: Some(formatting::round3(score_sum / suite_jobs.len() as f64)),
		unsupported_claim_count,
		wrong_result_count,
		stale_answer_count,
		conflict_detection_count,
		update_rationale_available_count,
		temporal_validity_not_encoded_count,
		history_readback_encoded_count,
		expected_evidence_recall: Some(summary::expected_evidence_recall_for_jobs(&suite_jobs)),
		irrelevant_context_ratio: Some(summary::irrelevant_context_ratio_for_jobs(&suite_jobs)),
		trace_explainability_count,
		reason: suite_reason(status, suite_jobs.len()),
	}
}

fn suite_reason(status: TypedStatus, encoded_job_count: usize) -> String {
	match status {
		TypedStatus::Pass => format!("All {encoded_job_count} encoded job(s) passed."),
		TypedStatus::UnsupportedClaim =>
			"At least one encoded job produced an unsupported claim.".to_string(),
		TypedStatus::WrongResult => "At least one encoded job returned a wrong result.".to_string(),
		TypedStatus::LifecycleFail =>
			"At least one encoded lifecycle-scored job failed lifecycle behavior.".to_string(),
		TypedStatus::Incomplete => "At least one encoded job could not complete.".to_string(),
		TypedStatus::Blocked => "At least one encoded job is blocked.".to_string(),
		TypedStatus::NotEncoded =>
			if encoded_job_count == 0 {
				NOT_ENCODED_REASON.to_string()
			} else {
				"At least one encoded fixture declares a not_encoded limitation.".to_string()
			},
	}
}
