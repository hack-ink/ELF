use crate::scoring::{
	ConsolidationJobReport, DimensionScoreReport, EvolutionJobReport, FailureCounts, JobScoring,
	MemorySummaryJobMetrics, ProactiveBriefJobMetrics, RealWorldJob, ScheduledMemoryJobMetrics,
	TypedStatus, WorkContinuityJobMetrics,
};

pub(super) fn apply_memory_summary_failure_counts(
	counts: &mut FailureCounts,
	metrics: Option<&MemorySummaryJobMetrics>,
) {
	let Some(metrics) = metrics else {
		return;
	};

	counts.memory_summary_invalid_current_entries = metrics.invalid_top_of_mind_count;
	counts.memory_summary_untraced_entries = metrics.untraced_entry_count;
	counts.memory_summary_missing_freshness =
		metrics.entry_count.saturating_sub(metrics.freshness_marker_count);
	counts.memory_summary_missing_rationale =
		metrics.entry_count.saturating_sub(metrics.rationale_count);
	counts.memory_summary_missing_categories = metrics.missing_required_category_count;
	counts.memory_summary_unsupported_current_entries = metrics.unsupported_current_entry_count;
}

pub(super) fn apply_proactive_brief_failure_counts(
	counts: &mut FailureCounts,
	metrics: Option<&ProactiveBriefJobMetrics>,
) {
	let Some(metrics) = metrics else {
		return;
	};

	counts.proactive_brief_invalid_current_suggestions = metrics.invalid_current_suggestion_count;
	counts.proactive_brief_untraced_suggestions = metrics.untraced_suggestion_count;
	counts.proactive_brief_missing_freshness =
		metrics.suggestion_count.saturating_sub(metrics.freshness_marker_count);
	counts.proactive_brief_missing_action_rationale =
		metrics.suggestion_count.saturating_sub(metrics.action_rationale_count);
	counts.proactive_brief_missing_kinds = metrics.missing_required_suggestion_kind_count;
	counts.proactive_brief_unsupported_current_suggestions =
		metrics.unsupported_current_suggestion_count;
	counts.proactive_brief_tombstone_violations = metrics.tombstone_violation_count;
}

pub(super) fn apply_scheduled_memory_failure_counts(
	counts: &mut FailureCounts,
	metrics: Option<&ScheduledMemoryJobMetrics>,
) {
	let Some(metrics) = metrics else {
		return;
	};

	counts.scheduled_memory_invalid_current_outputs = metrics.invalid_current_output_count;
	counts.scheduled_memory_untraced_outputs = metrics.untraced_output_count;
	counts.scheduled_memory_missing_freshness =
		metrics.output_count.saturating_sub(metrics.freshness_marker_count);
	counts.scheduled_memory_missing_action_rationale =
		metrics.output_count.saturating_sub(metrics.action_rationale_count);
	counts.scheduled_memory_missing_task_kinds = metrics.missing_required_task_kind_count;
	counts.scheduled_memory_unsupported_current_outputs = metrics.unsupported_current_output_count;
	counts.scheduled_memory_tombstone_violations = metrics.tombstone_violation_count;
	counts.scheduled_memory_missing_trace =
		metrics.trace_required_count.saturating_sub(metrics.trace_complete_count);
	counts.source_mutations += metrics.source_mutation_count;
}

pub(super) fn apply_work_continuity_failure_counts(
	counts: &mut FailureCounts,
	metrics: Option<&WorkContinuityJobMetrics>,
) {
	let Some(metrics) = metrics else {
		return;
	};

	counts.work_continuity_reset_resume_missing =
		metrics.reset_resume_required_count.saturating_sub(metrics.reset_resume_success_count);
	counts.work_continuity_decision_rationale_missing = metrics
		.decision_rationale_required_count
		.saturating_sub(metrics.decision_rationale_recalled_count);
	counts.work_continuity_rejected_option_unsuppressed = metrics
		.rejected_option_required_count
		.saturating_sub(metrics.rejected_option_suppressed_count);
	counts.work_continuity_rejected_option_resurrection =
		metrics.rejected_option_resurrection_count;
	counts.work_continuity_explicit_next_step_missing = metrics
		.explicit_next_step_required_count
		.saturating_sub(metrics.explicit_next_step_correct_count);
	counts.work_continuity_explicit_next_step_extra = metrics
		.explicit_next_step_returned_count
		.saturating_sub(metrics.explicit_next_step_correct_count);
	counts.work_continuity_inferred_step_unlabeled = metrics
		.inferred_next_step_required_count
		.saturating_sub(metrics.inferred_next_step_labeled_count);
	counts.work_continuity_inferred_step_as_instruction = metrics.inferred_step_instruction_count;
	counts.work_continuity_handoff_source_ref_missing = metrics
		.handoff_source_ref_required_count
		.saturating_sub(metrics.handoff_source_ref_covered_count);
	counts.work_continuity_redaction_missing =
		metrics.redaction_required_count.saturating_sub(metrics.redaction_applied_count);
	counts.work_continuity_sensitive_marker_persistence =
		metrics.sensitive_marker_persistence_count;
	counts.work_continuity_janitor_false_promotion = metrics.janitor_false_promotion_count;
	counts.work_continuity_journal_only_authority_claim =
		metrics.journal_only_authority_claim_count;
}

pub(super) fn score_declared_job(
	job: &RealWorldJob,
	status: TypedStatus,
	trap_ids_used: Vec<String>,
	evolution: Option<EvolutionJobReport>,
	consolidation: Option<ConsolidationJobReport>,
) -> JobScoring {
	JobScoring {
		status,
		normalized_score: 0.0,
		hard_fail_hits: Vec::new(),
		unsupported_claims: Vec::new(),
		wrong_result_count: 0,
		knowledge: None,
		trap_ids_used,
		dimension_scores: declared_not_encoded_dimension_scores(job),
		reason: job
			.encoding
			.reason
			.clone()
			.unwrap_or_else(|| "Job did not reach a runnable scoring state.".to_string()),
		evolution,
		consolidation,
		memory_summary: None,
		proactive_brief: None,
		scheduled_memory: None,
		work_continuity: None,
	}
}

pub(super) fn wrong_result_count(counts: &FailureCounts) -> usize {
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

pub(super) fn operator_debug_failure_counts(job: &RealWorldJob) -> FailureCounts {
	let Some(debug) = &job.operator_debug else {
		return FailureCounts {
			operator_debug_missing: usize::from(job.suite == "operator_debugging_ux"),
			..FailureCounts::default()
		};
	};

	FailureCounts {
		operator_debug_raw_sql: usize::from(debug.raw_sql_needed),
		operator_debug_trace_gaps: usize::from(debug.trace_completeness != "complete"),
		operator_debug_repair_unclear: usize::from(debug.repair_action_clarity != "clear"),
		..FailureCounts::default()
	}
}

pub(super) fn wrong_result_signal_count(counts: &FailureCounts) -> usize {
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

pub(super) fn work_continuity_wrong_result_count(counts: &FailureCounts) -> usize {
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

fn declared_not_encoded_dimension_scores(job: &RealWorldJob) -> Vec<DimensionScoreReport> {
	job.scoring_rubric
		.dimensions
		.iter()
		.map(|(dimension_id, dimension)| DimensionScoreReport {
			dimension: dimension_id.clone(),
			score: 0.0,
			max_points: dimension.max_points,
			weight: dimension.weight,
		})
		.collect()
}
