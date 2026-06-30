use crate::feature_metrics::{ProducedAnswer, WorkContinuityObserved, work_continuity::collectors};

pub(in crate::feature_metrics) fn work_continuity_observed(
	answer: &ProducedAnswer,
) -> WorkContinuityObserved<'_> {
	WorkContinuityObserved {
		reset_resume_entry_ids: collectors::work_journal_reset_resume_entry_ids(answer),
		decision_rationale_evidence_ids: collectors::work_journal_decision_rationale_evidence_ids(
			answer,
		),
		rejected_options: collectors::work_journal_rejected_options(answer),
		explicit_next_steps: collectors::work_journal_explicit_next_steps(answer),
		inferred_next_steps: collectors::work_journal_inferred_next_steps(answer),
		handoff_source_refs: collectors::work_journal_handoff_source_refs(answer),
		redacted_marker_ids: collectors::work_journal_redacted_marker_ids(answer),
		janitor_candidates: collectors::work_journal_janitor_candidates(answer),
	}
}
