use crate::scoring::FailureCounts;

pub(in crate::scoring) fn wrong_result_count(counts: &FailureCounts) -> usize {
	counts.missing_claims
		+ counts.forbidden_claims
		+ counts.missing_evidence
		+ counts.trap_uses
		+ counts.operator_debug_missing
		+ counts.operator_debug_raw_sql
		+ counts.operator_debug_trace_gaps
		+ counts.operator_debug_repair_unclear
		+ counts.conflict_detection_missing
		+ counts.update_rationale_missing
		+ counts.proposal_usefulness_failures
		+ counts.lineage_failures
		+ counts.review_action_failures
		+ counts.memory_summary_invalid_current_entries
		+ counts.memory_summary_untraced_entries
		+ counts.memory_summary_missing_freshness
		+ counts.memory_summary_missing_rationale
		+ counts.memory_summary_missing_categories
		+ counts.memory_summary_unsupported_current_entries
		+ counts.proactive_brief_invalid_current_suggestions
		+ counts.proactive_brief_untraced_suggestions
		+ counts.proactive_brief_missing_freshness
		+ counts.proactive_brief_missing_action_rationale
		+ counts.proactive_brief_missing_kinds
		+ counts.proactive_brief_unsupported_current_suggestions
		+ counts.proactive_brief_tombstone_violations
		+ counts.scheduled_memory_invalid_current_outputs
		+ counts.scheduled_memory_untraced_outputs
		+ counts.scheduled_memory_missing_freshness
		+ counts.scheduled_memory_missing_action_rationale
		+ counts.scheduled_memory_missing_task_kinds
		+ counts.scheduled_memory_unsupported_current_outputs
		+ counts.scheduled_memory_tombstone_violations
		+ counts.scheduled_memory_missing_trace
		+ work_continuity_wrong_result_count(counts)
		+ counts.untraced_page_sections
		+ counts.missed_stale_findings
		+ counts.rebuild_failures
		+ counts.page_usefulness_failures
}

pub(in crate::scoring) fn wrong_result_signal_count(counts: &FailureCounts) -> usize {
	counts.missing_claims
		+ counts.forbidden_claims
		+ counts.missing_evidence
		+ counts.trap_uses
		+ counts.operator_debug_missing
		+ counts.operator_debug_raw_sql
		+ counts.operator_debug_trace_gaps
		+ counts.operator_debug_repair_unclear
		+ counts.conflict_detection_missing
		+ counts.update_rationale_missing
		+ counts.proposal_usefulness_failures
		+ counts.lineage_failures
		+ counts.review_action_failures
		+ counts.memory_summary_invalid_current_entries
		+ counts.memory_summary_untraced_entries
		+ counts.memory_summary_missing_freshness
		+ counts.memory_summary_missing_rationale
		+ counts.memory_summary_missing_categories
		+ counts.memory_summary_unsupported_current_entries
		+ counts.proactive_brief_invalid_current_suggestions
		+ counts.proactive_brief_untraced_suggestions
		+ counts.proactive_brief_missing_freshness
		+ counts.proactive_brief_missing_action_rationale
		+ counts.proactive_brief_missing_kinds
		+ counts.proactive_brief_unsupported_current_suggestions
		+ counts.proactive_brief_tombstone_violations
		+ counts.scheduled_memory_invalid_current_outputs
		+ counts.scheduled_memory_untraced_outputs
		+ counts.scheduled_memory_missing_freshness
		+ counts.scheduled_memory_missing_action_rationale
		+ counts.scheduled_memory_missing_task_kinds
		+ counts.scheduled_memory_unsupported_current_outputs
		+ counts.scheduled_memory_tombstone_violations
		+ counts.scheduled_memory_missing_trace
		+ work_continuity_wrong_result_count(counts)
		+ counts.untraced_page_sections
		+ counts.missed_stale_findings
		+ counts.rebuild_failures
		+ counts.page_usefulness_failures
}

fn work_continuity_wrong_result_count(counts: &FailureCounts) -> usize {
	counts.work_continuity_reset_resume_missing
		+ counts.work_continuity_decision_rationale_missing
		+ counts.work_continuity_rejected_option_unsuppressed
		+ counts.work_continuity_rejected_option_resurrection
		+ counts.work_continuity_explicit_next_step_missing
		+ counts.work_continuity_explicit_next_step_extra
		+ counts.work_continuity_inferred_step_unlabeled
		+ counts.work_continuity_inferred_step_as_instruction
		+ counts.work_continuity_handoff_source_ref_missing
		+ counts.work_continuity_redaction_missing
		+ counts.work_continuity_sensitive_marker_persistence
		+ counts.work_continuity_janitor_false_promotion
		+ counts.work_continuity_journal_only_authority_claim
}
