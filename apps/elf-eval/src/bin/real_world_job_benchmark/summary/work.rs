use crate::summary::{self, JobReport, WorkContinuitySummaryReport};

pub(super) fn work_continuity_summary_impl(
	jobs: &[JobReport],
) -> Option<WorkContinuitySummaryReport> {
	let work_jobs = jobs.iter().filter_map(|job| job.work_continuity.as_ref()).collect::<Vec<_>>();

	if work_jobs.is_empty() {
		return None;
	}

	let reset_resume_required_count =
		work_jobs.iter().map(|metrics| metrics.reset_resume_required_count).sum();
	let reset_resume_success_count =
		work_jobs.iter().map(|metrics| metrics.reset_resume_success_count).sum();
	let decision_rationale_required_count =
		work_jobs.iter().map(|metrics| metrics.decision_rationale_required_count).sum();
	let decision_rationale_recalled_count =
		work_jobs.iter().map(|metrics| metrics.decision_rationale_recalled_count).sum();
	let rejected_option_required_count =
		work_jobs.iter().map(|metrics| metrics.rejected_option_required_count).sum();
	let rejected_option_suppressed_count =
		work_jobs.iter().map(|metrics| metrics.rejected_option_suppressed_count).sum();
	let explicit_next_step_returned_count =
		work_jobs.iter().map(|metrics| metrics.explicit_next_step_returned_count).sum();
	let explicit_next_step_correct_count =
		work_jobs.iter().map(|metrics| metrics.explicit_next_step_correct_count).sum();
	let inferred_next_step_required_count =
		work_jobs.iter().map(|metrics| metrics.inferred_next_step_required_count).sum();
	let inferred_next_step_labeled_count =
		work_jobs.iter().map(|metrics| metrics.inferred_next_step_labeled_count).sum();
	let handoff_source_ref_required_count =
		work_jobs.iter().map(|metrics| metrics.handoff_source_ref_required_count).sum();
	let handoff_source_ref_covered_count =
		work_jobs.iter().map(|metrics| metrics.handoff_source_ref_covered_count).sum();
	let redaction_required_count =
		work_jobs.iter().map(|metrics| metrics.redaction_required_count).sum();
	let redaction_applied_count =
		work_jobs.iter().map(|metrics| metrics.redaction_applied_count).sum();
	let janitor_candidate_count =
		work_jobs.iter().map(|metrics| metrics.janitor_candidate_count).sum();
	let janitor_false_promotion_count =
		work_jobs.iter().map(|metrics| metrics.janitor_false_promotion_count).sum();

	Some(WorkContinuitySummaryReport {
		job_count: work_jobs.len(),
		readback_count: work_jobs.iter().map(|metrics| metrics.readback_count).sum(),
		entry_count: work_jobs.iter().map(|metrics| metrics.entry_count).sum(),
		reset_resume_required_count,
		reset_resume_success_count,
		reset_resume_success_rate: summary::ratio(
			reset_resume_success_count,
			reset_resume_required_count,
		),
		decision_rationale_required_count,
		decision_rationale_recalled_count,
		decision_rationale_recall_rate: summary::ratio(
			decision_rationale_recalled_count,
			decision_rationale_required_count,
		),
		rejected_option_required_count,
		rejected_option_suppressed_count,
		rejected_option_resurrection_count: work_jobs
			.iter()
			.map(|metrics| metrics.rejected_option_resurrection_count)
			.sum(),
		rejected_option_suppression_rate: summary::ratio(
			rejected_option_suppressed_count,
			rejected_option_required_count,
		),
		explicit_next_step_required_count: work_jobs
			.iter()
			.map(|metrics| metrics.explicit_next_step_required_count)
			.sum(),
		explicit_next_step_returned_count,
		explicit_next_step_correct_count,
		explicit_next_step_precision: summary::ratio_or(
			explicit_next_step_correct_count,
			explicit_next_step_returned_count,
			1.0,
		),
		inferred_next_step_required_count,
		inferred_next_step_labeled_count,
		inferred_step_instruction_count: work_jobs
			.iter()
			.map(|metrics| metrics.inferred_step_instruction_count)
			.sum(),
		inferred_next_step_labeling_rate: summary::ratio(
			inferred_next_step_labeled_count,
			inferred_next_step_required_count,
		),
		handoff_source_ref_required_count,
		handoff_source_ref_covered_count,
		handoff_source_ref_coverage: summary::ratio(
			handoff_source_ref_covered_count,
			handoff_source_ref_required_count,
		),
		redaction_required_count,
		redaction_applied_count,
		sensitive_marker_persistence_count: work_jobs
			.iter()
			.map(|metrics| metrics.sensitive_marker_persistence_count)
			.sum(),
		redaction_rate: summary::ratio(redaction_applied_count, redaction_required_count),
		janitor_candidate_count,
		janitor_false_promotion_count,
		janitor_false_promotion_rate: summary::ratio(
			janitor_false_promotion_count,
			janitor_candidate_count,
		),
		journal_only_authority_claim_count: work_jobs
			.iter()
			.map(|metrics| metrics.journal_only_authority_claim_count)
			.sum(),
	})
}
