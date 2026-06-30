use crate::feature_metrics::{
	self, ProducedAnswer, RealWorldJob, WorkContinuityExpectation, WorkContinuityJobMetrics,
	WorkContinuityObserved,
	work_continuity::{collectors, observed},
};

pub(in crate::feature_metrics) fn work_continuity_metrics_impl(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
) -> Option<WorkContinuityJobMetrics> {
	if job.work_continuity.is_none() && answer.work_journal_readbacks.is_empty() {
		return None;
	}

	let expectation = job.work_continuity.as_ref();
	let observed = observed::work_continuity_observed(answer);
	let mut metrics = initial_work_continuity_metrics(expectation, answer);

	if let Some(expected) = expectation {
		apply_expected_work_continuity_counts(&mut metrics, expected, &observed);
	}

	apply_observed_work_continuity_counts(&mut metrics, answer, &observed);
	apply_work_continuity_rates(&mut metrics);

	Some(metrics)
}

fn initial_work_continuity_metrics(
	expectation: Option<&WorkContinuityExpectation>,
	answer: &ProducedAnswer,
) -> WorkContinuityJobMetrics {
	WorkContinuityJobMetrics {
		readback_count: answer.work_journal_readbacks.len(),
		entry_count: answer
			.work_journal_readbacks
			.iter()
			.map(|readback| readback.items.len())
			.sum(),
		reset_resume_required_count: expectation
			.map_or(0, |expected| expected.required_reset_resume_entry_ids.len()),
		decision_rationale_required_count: expectation
			.map_or(0, |expected| expected.required_decision_rationale_evidence_ids.len()),
		rejected_option_required_count: expectation
			.map_or(0, |expected| expected.required_rejected_option_ids.len()),
		explicit_next_step_required_count: expectation
			.map_or(0, |expected| expected.required_explicit_next_step_ids.len()),
		inferred_next_step_required_count: expectation
			.map_or(0, |expected| expected.required_inferred_next_step_ids.len()),
		handoff_source_ref_required_count: expectation
			.map_or(0, |expected| expected.required_handoff_source_ref_ids.len()),
		redaction_required_count: expectation
			.map_or(0, |expected| expected.required_redaction_marker_ids.len()),
		janitor_candidate_count: expectation
			.map_or(0, |expected| expected.required_janitor_candidate_ids.len()),
		..WorkContinuityJobMetrics::default()
	}
}

fn apply_expected_work_continuity_counts(
	metrics: &mut WorkContinuityJobMetrics,
	expected: &WorkContinuityExpectation,
	observed: &WorkContinuityObserved<'_>,
) {
	metrics.reset_resume_success_count = expected
		.required_reset_resume_entry_ids
		.iter()
		.filter(|entry_id| observed.reset_resume_entry_ids.contains(entry_id.as_str()))
		.count();
	metrics.decision_rationale_recalled_count = expected
		.required_decision_rationale_evidence_ids
		.iter()
		.filter(|evidence_id| {
			observed.decision_rationale_evidence_ids.contains(evidence_id.as_str())
		})
		.count();
	metrics.rejected_option_suppressed_count = expected
		.required_rejected_option_ids
		.iter()
		.filter(|option_id| {
			observed
				.rejected_options
				.iter()
				.any(|option| option.option_id == **option_id && !option.resurrected_as_current)
		})
		.count();
	metrics.explicit_next_step_correct_count = expected
		.required_explicit_next_step_ids
		.iter()
		.filter(|step_id| {
			observed.explicit_next_steps.iter().any(|step| {
				step.step_id == **step_id && step.label == "explicit" && step.instruction
			})
		})
		.count();
	metrics.inferred_next_step_labeled_count = expected
		.required_inferred_next_step_ids
		.iter()
		.filter(|step_id| {
			observed.inferred_next_steps.iter().any(|step| {
				step.step_id == **step_id && step.label == "inferred" && !step.instruction
			})
		})
		.count();
	metrics.handoff_source_ref_covered_count = expected
		.required_handoff_source_ref_ids
		.iter()
		.filter(|source_ref| observed.handoff_source_refs.contains(source_ref.as_str()))
		.count();
	metrics.redaction_applied_count = expected
		.required_redaction_marker_ids
		.iter()
		.filter(|marker_id| observed.redacted_marker_ids.contains(marker_id.as_str()))
		.count();
}

fn apply_observed_work_continuity_counts(
	metrics: &mut WorkContinuityJobMetrics,
	answer: &ProducedAnswer,
	observed: &WorkContinuityObserved<'_>,
) {
	metrics.janitor_candidate_count =
		metrics.janitor_candidate_count.max(observed.janitor_candidates.len());
	metrics.janitor_false_promotion_count = observed
		.janitor_candidates
		.iter()
		.filter(|candidate| candidate.promoted_to_memory || !candidate.review_required)
		.count();
	metrics.explicit_next_step_returned_count = observed.explicit_next_steps.len();
	metrics.rejected_option_resurrection_count =
		observed.rejected_options.iter().filter(|option| option.resurrected_as_current).count();
	metrics.inferred_step_instruction_count =
		observed.inferred_next_steps.iter().filter(|step| step.instruction).count();
	metrics.sensitive_marker_persistence_count = answer
		.work_journal_readbacks
		.iter()
		.flat_map(|readback| readback.items.iter())
		.map(|entry| entry.redaction_audit.persisted_sensitive_marker_ids.len())
		.sum();
	metrics.journal_only_authority_claim_count = answer
		.work_journal_readbacks
		.iter()
		.map(collectors::work_journal_authority_claim_count)
		.sum();
}

fn apply_work_continuity_rates(metrics: &mut WorkContinuityJobMetrics) {
	metrics.reset_resume_success_rate = feature_metrics::ratio(
		metrics.reset_resume_success_count,
		metrics.reset_resume_required_count,
	);
	metrics.decision_rationale_recall_rate = feature_metrics::ratio(
		metrics.decision_rationale_recalled_count,
		metrics.decision_rationale_required_count,
	);
	metrics.rejected_option_suppression_rate = feature_metrics::ratio(
		metrics.rejected_option_suppressed_count,
		metrics.rejected_option_required_count,
	);
	metrics.explicit_next_step_precision = feature_metrics::ratio_or(
		metrics.explicit_next_step_correct_count,
		metrics.explicit_next_step_returned_count,
		usize::from(metrics.explicit_next_step_required_count == 0) as f64,
	);
	metrics.inferred_next_step_labeling_rate = feature_metrics::ratio(
		metrics.inferred_next_step_labeled_count,
		metrics.inferred_next_step_required_count,
	);
	metrics.handoff_source_ref_coverage = feature_metrics::ratio(
		metrics.handoff_source_ref_covered_count,
		metrics.handoff_source_ref_required_count,
	);
	metrics.redaction_rate =
		feature_metrics::ratio(metrics.redaction_applied_count, metrics.redaction_required_count);
	metrics.janitor_false_promotion_rate = feature_metrics::ratio(
		metrics.janitor_false_promotion_count,
		metrics.janitor_candidate_count,
	);
}
