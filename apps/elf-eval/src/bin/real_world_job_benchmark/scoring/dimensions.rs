use super::*;

pub(super) fn dimension_scores(
	job: &RealWorldJob,
	counts: &FailureCounts,
) -> Vec<DimensionScoreReport> {
	job.scoring_rubric
		.dimensions
		.iter()
		.map(|(dimension_id, dimension)| DimensionScoreReport {
			dimension: dimension_id.clone(),
			score: dimension_score(dimension_id, dimension.max_points, counts),
			max_points: dimension.max_points,
			weight: dimension.weight,
		})
		.collect()
}

fn dimension_score(dimension_id: &str, max_points: f64, counts: &FailureCounts) -> f64 {
	let failed = match dimension_id {
		"answer_correctness" | "workflow_helpfulness" =>
			counts.missing_claims > 0
				|| counts.forbidden_claims > 0
				|| counts.operator_debug_repair_unclear > 0
				|| counts.conflict_detection_missing > 0
				|| counts.proposal_usefulness_failures > 0
				|| counts.review_action_failures > 0
				|| counts.memory_summary_invalid_current_entries > 0
				|| counts.memory_summary_missing_categories > 0
				|| counts.memory_summary_unsupported_current_entries > 0
				|| counts.proactive_brief_invalid_current_suggestions > 0
				|| counts.proactive_brief_missing_kinds > 0
				|| counts.proactive_brief_unsupported_current_suggestions > 0
				|| counts.proactive_brief_tombstone_violations > 0
				|| counts.scheduled_memory_invalid_current_outputs > 0
				|| counts.scheduled_memory_missing_task_kinds > 0
				|| counts.scheduled_memory_unsupported_current_outputs > 0
				|| counts.scheduled_memory_tombstone_violations > 0
				|| counts.scheduled_memory_missing_trace > 0
				|| counts.work_continuity_reset_resume_missing > 0
				|| counts.work_continuity_decision_rationale_missing > 0
				|| counts.work_continuity_rejected_option_unsuppressed > 0
				|| counts.work_continuity_rejected_option_resurrection > 0
				|| counts.work_continuity_explicit_next_step_missing > 0
				|| counts.work_continuity_explicit_next_step_extra > 0
				|| counts.work_continuity_inferred_step_unlabeled > 0
				|| counts.work_continuity_inferred_step_as_instruction > 0
				|| counts.work_continuity_janitor_false_promotion > 0
				|| counts.work_continuity_journal_only_authority_claim > 0
				|| counts.page_usefulness_failures > 0,
		"evidence_grounding" =>
			counts.missing_evidence > 0
				|| counts.unsupported_claims > 0
				|| counts.lineage_failures > 0
				|| counts.memory_summary_untraced_entries > 0
				|| counts.proactive_brief_untraced_suggestions > 0
				|| counts.scheduled_memory_untraced_outputs > 0
				|| counts.scheduled_memory_missing_trace > 0
				|| counts.work_continuity_decision_rationale_missing > 0
				|| counts.work_continuity_handoff_source_ref_missing > 0
				|| counts.work_continuity_redaction_missing > 0
				|| counts.work_continuity_sensitive_marker_persistence > 0
				|| counts.untraced_page_sections > 0,
		"trap_avoidance" =>
			counts.trap_uses > 0
				|| counts.memory_summary_invalid_current_entries > 0
				|| counts.proactive_brief_invalid_current_suggestions > 0
				|| counts.proactive_brief_tombstone_violations > 0
				|| counts.scheduled_memory_invalid_current_outputs > 0
				|| counts.scheduled_memory_tombstone_violations > 0
				|| counts.work_continuity_rejected_option_resurrection > 0
				|| counts.work_continuity_sensitive_marker_persistence > 0
				|| counts.missed_stale_findings > 0,
		"uncertainty_handling" =>
			counts.unsupported_claims > 0
				|| counts.memory_summary_unsupported_current_entries > 0
				|| counts.proactive_brief_unsupported_current_suggestions > 0
				|| counts.scheduled_memory_unsupported_current_outputs > 0
				|| counts.work_continuity_journal_only_authority_claim > 0,
		"lifecycle_behavior" =>
			counts.stale_answers > 0
				|| counts.conflict_detection_missing > 0
				|| counts.update_rationale_missing > 0
				|| counts.source_mutations > 0
				|| counts.memory_summary_invalid_current_entries > 0
				|| counts.memory_summary_missing_freshness > 0
				|| counts.memory_summary_missing_rationale > 0
				|| counts.memory_summary_unsupported_current_entries > 0
				|| counts.proactive_brief_invalid_current_suggestions > 0
				|| counts.proactive_brief_missing_freshness > 0
				|| counts.proactive_brief_missing_action_rationale > 0
				|| counts.proactive_brief_unsupported_current_suggestions > 0
				|| counts.proactive_brief_tombstone_violations > 0
				|| counts.scheduled_memory_invalid_current_outputs > 0
				|| counts.scheduled_memory_missing_freshness > 0
				|| counts.scheduled_memory_missing_action_rationale > 0
				|| counts.scheduled_memory_unsupported_current_outputs > 0
				|| counts.scheduled_memory_tombstone_violations > 0
				|| counts.scheduled_memory_missing_trace > 0
				|| counts.work_continuity_reset_resume_missing > 0
				|| counts.work_continuity_inferred_step_as_instruction > 0
				|| counts.work_continuity_janitor_false_promotion > 0
				|| counts.work_continuity_journal_only_authority_claim > 0
				|| counts.rebuild_failures > 0,
		"source_immutability" => counts.source_mutations > 0,
		"proposal_usefulness" => counts.proposal_usefulness_failures > 0,
		"lineage_completeness" => counts.lineage_failures > 0,
		"review_action_correctness" => counts.review_action_failures > 0,
		"debuggability" =>
			counts.missing_claims > 0
				|| counts.unsupported_claims > 0
				|| counts.operator_debug_missing > 0
				|| counts.operator_debug_raw_sql > 0
				|| counts.operator_debug_trace_gaps > 0
				|| counts.scheduled_memory_missing_trace > 0
				|| counts.work_continuity_reset_resume_missing > 0,
		"trace_readback" => counts.scheduled_memory_missing_trace > 0,
		"latency_resource" => counts.latency_violations > 0,
		"personalization_fit" | "ownership_correctness" =>
			counts.missing_claims > 0 || counts.unsupported_claims > 0,
		_ => counts.missing_claims > 0 || counts.unsupported_claims > 0 || counts.trap_uses > 0,
	};

	if failed { 0.0 } else { max_points }
}

pub(super) fn latency_violations(job: &RealWorldJob, answer: &ProducedAnswer) -> usize {
	let Some(max_latency_ms) = latency_threshold_ms(job) else {
		return 0;
	};
	let Some(latency_ms) = answer.latency_ms else {
		return 1;
	};

	usize::from(latency_ms > max_latency_ms)
}

fn latency_threshold_ms(job: &RealWorldJob) -> Option<f64> {
	job.scoring_rubric
		.dimensions
		.get("latency_resource")
		.and_then(|dimension| dimension.criteria.get("max_latency_ms"))
		.and_then(Value::as_f64)
}

pub(super) fn normalized_score(scores: &[DimensionScoreReport]) -> f64 {
	let total_weight = scores.iter().map(|score| score.weight).sum::<f64>();

	if total_weight == 0.0 {
		return 0.0;
	}

	scores.iter().map(|score| (score.score / score.max_points) * score.weight).sum::<f64>()
		/ total_weight
}

pub(super) fn job_status(
	normalized_score: f64,
	pass_threshold: f64,
	wrong_result_count: usize,
	unsupported_claim_count: usize,
	source_mutation_count: usize,
	blocking_executable_gap_count: usize,
) -> TypedStatus {
	if unsupported_claim_count > 0 {
		TypedStatus::UnsupportedClaim
	} else if source_mutation_count > 0 {
		TypedStatus::LifecycleFail
	} else if blocking_executable_gap_count > 0 {
		TypedStatus::Blocked
	} else if wrong_result_count > 0 {
		TypedStatus::WrongResult
	} else if normalized_score >= pass_threshold {
		TypedStatus::Pass
	} else {
		TypedStatus::WrongResult
	}
}

pub(super) fn job_reason(
	status: TypedStatus,
	counts: &FailureCounts,
	normalized_score: f64,
) -> String {
	let wrong_result_signal_count = wrong_result_signal_count(counts);

	match status {
		TypedStatus::Pass => format!("Job passed with normalized_score {normalized_score:.3}."),
		TypedStatus::UnsupportedClaim => format!(
			"Job produced {} unsupported claim(s), {} wrong-result signal(s), {} latency violation(s), and normalized_score {normalized_score:.3}.",
			counts.unsupported_claims, wrong_result_signal_count, counts.latency_violations
		),
		TypedStatus::WrongResult => format!(
			"Job produced {} wrong-result signal(s), {} latency violation(s), and normalized_score {normalized_score:.3}.",
			wrong_result_signal_count, counts.latency_violations
		),
		TypedStatus::LifecycleFail => format!(
			"Job produced {} source mutation(s) and normalized_score {normalized_score:.3}.",
			counts.source_mutations
		),
		TypedStatus::Blocked => format!(
			"Job has {} blocking executable gap(s) and normalized_score {normalized_score:.3}.",
			counts.blocking_executable_gaps
		),
		_ => "Job did not reach a runnable scoring state.".to_string(),
	}
}
