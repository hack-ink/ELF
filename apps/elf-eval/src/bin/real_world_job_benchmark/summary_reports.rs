use crate::{CostReport, Deserialize, Serialize, TypedStatus};

#[derive(Debug, Default, Deserialize, Serialize)]
pub(super) struct ReportSummary {
	pub(super) job_count: usize,
	pub(super) encoded_suite_count: usize,
	pub(super) pass: usize,
	pub(super) wrong_result: usize,
	pub(super) lifecycle_fail: usize,
	pub(super) incomplete: usize,
	pub(super) blocked: usize,
	pub(super) not_encoded: usize,
	pub(super) unsupported_claim: usize,
	pub(super) unsupported_claim_count: usize,
	pub(super) wrong_result_count: usize,
	#[serde(default)]
	pub(super) stale_answer_count: usize,
	#[serde(default)]
	pub(super) conflict_detection_count: usize,
	#[serde(default)]
	pub(super) update_rationale_available_count: usize,
	#[serde(default)]
	pub(super) temporal_validity_not_encoded_count: usize,
	#[serde(default)]
	pub(super) history_readback_encoded_count: usize,
	pub(super) expected_evidence_total: usize,
	pub(super) expected_evidence_matched: usize,
	pub(super) expected_evidence_recall: f64,
	pub(super) irrelevant_context_count: usize,
	pub(super) irrelevant_context_ratio: f64,
	pub(super) trace_explainability_count: usize,
	pub(super) wrong_result_stage_attribution_count: usize,
	pub(super) mean_score: f64,
	pub(super) mean_latency_ms: Option<f64>,
	pub(super) total_cost: Option<CostReport>,
	#[serde(default)]
	pub(super) evidence_required_count: usize,
	#[serde(default)]
	pub(super) evidence_covered_count: usize,
	#[serde(default)]
	pub(super) evidence_coverage: f64,
	#[serde(default)]
	pub(super) source_ref_required_count: usize,
	#[serde(default)]
	pub(super) source_ref_covered_count: usize,
	#[serde(default)]
	pub(super) source_ref_coverage: f64,
	#[serde(default)]
	pub(super) quote_required_count: usize,
	#[serde(default)]
	pub(super) quote_covered_count: usize,
	#[serde(default)]
	pub(super) quote_coverage: f64,
	#[serde(default)]
	pub(super) stale_retrieval_count: usize,
	#[serde(default)]
	pub(super) scope_check_count: usize,
	#[serde(default)]
	pub(super) scope_correct_count: usize,
	#[serde(default)]
	pub(super) scope_correctness: f64,
	#[serde(default)]
	pub(super) scope_violation_count: usize,
	#[serde(default)]
	pub(super) redaction_leak_count: usize,
	#[serde(default)]
	pub(super) qdrant_rebuild_case_count: usize,
	#[serde(default)]
	pub(super) qdrant_rebuild_pass_count: usize,
	#[serde(default)]
	pub(super) operator_debug_job_count: usize,
	#[serde(default)]
	pub(super) raw_sql_needed_count: usize,
	#[serde(default)]
	pub(super) trace_incomplete_count: usize,
	#[serde(default)]
	pub(super) operator_ux_gap_count: usize,
	#[serde(default)]
	pub(super) consolidation: ConsolidationSummaryReport,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) memory_summary: Option<MemorySummaryReport>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) proactive_brief: Option<ProactiveBriefSummaryReport>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) scheduled_memory: Option<ScheduledMemorySummaryReport>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) work_continuity: Option<WorkContinuitySummaryReport>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) knowledge: Option<KnowledgeSummary>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub(super) struct ConsolidationSummaryReport {
	pub(super) proposal_count: usize,
	pub(super) proposal_usefulness: Option<f64>,
	pub(super) lineage_completeness: Option<f64>,
	pub(super) review_action_correctness: Option<f64>,
	pub(super) source_mutation_count: usize,
	pub(super) proposal_unsupported_claim_count: usize,
	pub(super) executable_gap_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct MemorySummaryReport {
	pub(super) job_count: usize,
	pub(super) summary_count: usize,
	pub(super) entry_count: usize,
	pub(super) required_category_count: usize,
	pub(super) covered_required_category_count: usize,
	pub(super) missing_required_category_count: usize,
	pub(super) top_of_mind_count: usize,
	pub(super) background_count: usize,
	pub(super) stale_count: usize,
	pub(super) superseded_count: usize,
	pub(super) tombstone_count: usize,
	pub(super) derived_project_profile_count: usize,
	pub(super) source_ref_required_count: usize,
	pub(super) source_ref_entry_count: usize,
	pub(super) source_ref_coverage: f64,
	pub(super) freshness_marker_count: usize,
	pub(super) freshness_coverage: f64,
	pub(super) rationale_count: usize,
	pub(super) rationale_coverage: f64,
	pub(super) invalid_top_of_mind_count: usize,
	pub(super) untraced_entry_count: usize,
	pub(super) derived_with_source_or_unsupported_count: usize,
	pub(super) derived_missing_source_or_unsupported_count: usize,
	pub(super) unsupported_derived_entry_count: usize,
	pub(super) unsupported_current_entry_count: usize,
	pub(super) tombstone_ref_count: usize,
	pub(super) source_trace_selected_count: usize,
	pub(super) source_trace_dropped_count: usize,
	pub(super) source_trace_stale_count: usize,
	pub(super) source_trace_superseded_count: usize,
	pub(super) source_trace_tombstone_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ProactiveBriefSummaryReport {
	pub(super) job_count: usize,
	pub(super) brief_count: usize,
	pub(super) suggestion_count: usize,
	pub(super) required_suggestion_kind_count: usize,
	pub(super) covered_required_suggestion_kind_count: usize,
	pub(super) missing_required_suggestion_kind_count: usize,
	pub(super) evidence_ref_required_count: usize,
	pub(super) evidence_ref_suggestion_count: usize,
	pub(super) evidence_ref_coverage: f64,
	pub(super) freshness_marker_count: usize,
	pub(super) freshness_coverage: f64,
	pub(super) action_rationale_count: usize,
	pub(super) action_rationale_coverage: f64,
	pub(super) recommended_count: usize,
	pub(super) deferred_count: usize,
	pub(super) rejected_count: usize,
	pub(super) current_suggestion_count: usize,
	pub(super) non_current_suggestion_count: usize,
	pub(super) stale_warning_count: usize,
	pub(super) invalid_current_suggestion_count: usize,
	pub(super) untraced_suggestion_count: usize,
	pub(super) unsupported_current_suggestion_count: usize,
	pub(super) tombstone_violation_count: usize,
	pub(super) source_trace_selected_count: usize,
	pub(super) source_trace_dropped_count: usize,
	pub(super) source_trace_stale_count: usize,
	pub(super) source_trace_superseded_count: usize,
	pub(super) source_trace_tombstone_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ScheduledMemorySummaryReport {
	pub(super) job_count: usize,
	pub(super) task_run_count: usize,
	pub(super) output_count: usize,
	pub(super) required_task_kind_count: usize,
	pub(super) covered_required_task_kind_count: usize,
	pub(super) missing_required_task_kind_count: usize,
	pub(super) evidence_ref_required_count: usize,
	pub(super) evidence_ref_output_count: usize,
	pub(super) evidence_ref_coverage: f64,
	pub(super) freshness_marker_count: usize,
	pub(super) freshness_coverage: f64,
	pub(super) action_rationale_count: usize,
	pub(super) action_rationale_coverage: f64,
	pub(super) trace_required_count: usize,
	pub(super) trace_complete_count: usize,
	pub(super) trace_coverage: f64,
	pub(super) source_mutation_count: usize,
	pub(super) current_output_count: usize,
	pub(super) non_current_output_count: usize,
	pub(super) invalid_current_output_count: usize,
	pub(super) untraced_output_count: usize,
	pub(super) unsupported_current_output_count: usize,
	pub(super) tombstone_violation_count: usize,
	pub(super) source_trace_selected_count: usize,
	pub(super) source_trace_dropped_count: usize,
	pub(super) source_trace_stale_count: usize,
	pub(super) source_trace_superseded_count: usize,
	pub(super) source_trace_tombstone_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct WorkContinuitySummaryReport {
	pub(super) job_count: usize,
	pub(super) readback_count: usize,
	pub(super) entry_count: usize,
	pub(super) reset_resume_required_count: usize,
	pub(super) reset_resume_success_count: usize,
	pub(super) reset_resume_success_rate: f64,
	pub(super) decision_rationale_required_count: usize,
	pub(super) decision_rationale_recalled_count: usize,
	pub(super) decision_rationale_recall_rate: f64,
	pub(super) rejected_option_required_count: usize,
	pub(super) rejected_option_suppressed_count: usize,
	pub(super) rejected_option_resurrection_count: usize,
	pub(super) rejected_option_suppression_rate: f64,
	pub(super) explicit_next_step_required_count: usize,
	pub(super) explicit_next_step_returned_count: usize,
	pub(super) explicit_next_step_correct_count: usize,
	pub(super) explicit_next_step_precision: f64,
	pub(super) inferred_next_step_required_count: usize,
	pub(super) inferred_next_step_labeled_count: usize,
	pub(super) inferred_step_instruction_count: usize,
	pub(super) inferred_next_step_labeling_rate: f64,
	pub(super) handoff_source_ref_required_count: usize,
	pub(super) handoff_source_ref_covered_count: usize,
	pub(super) handoff_source_ref_coverage: f64,
	pub(super) redaction_required_count: usize,
	pub(super) redaction_applied_count: usize,
	pub(super) sensitive_marker_persistence_count: usize,
	pub(super) redaction_rate: f64,
	pub(super) janitor_candidate_count: usize,
	pub(super) janitor_false_promotion_count: usize,
	pub(super) janitor_false_promotion_rate: f64,
	pub(super) journal_only_authority_claim_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct KnowledgeSummary {
	pub(super) job_count: usize,
	pub(super) page_count: usize,
	pub(super) section_count: usize,
	pub(super) backlink_count: usize,
	pub(super) pages_with_backlinks: usize,
	pub(super) pages_with_version_diff: usize,
	pub(super) citation_coverage: f64,
	pub(super) stale_claim_detection: f64,
	pub(super) rebuild_determinism: f64,
	pub(super) backlink_coverage: f64,
	pub(super) version_diff_coverage: f64,
	pub(super) page_usefulness: f64,
	pub(super) unsupported_summary_count: usize,
	pub(super) untraced_section_count: usize,
	pub(super) allowed_variance_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct SuiteReport {
	pub(super) suite_id: String,
	pub(super) status: TypedStatus,
	pub(super) encoded_job_count: usize,
	pub(super) score_mean: Option<f64>,
	pub(super) unsupported_claim_count: usize,
	pub(super) wrong_result_count: usize,
	#[serde(default)]
	pub(super) stale_answer_count: usize,
	#[serde(default)]
	pub(super) conflict_detection_count: usize,
	#[serde(default)]
	pub(super) update_rationale_available_count: usize,
	#[serde(default)]
	pub(super) temporal_validity_not_encoded_count: usize,
	#[serde(default)]
	pub(super) history_readback_encoded_count: usize,
	pub(super) expected_evidence_recall: Option<f64>,
	pub(super) irrelevant_context_ratio: Option<f64>,
	pub(super) trace_explainability_count: usize,
	pub(super) reason: String,
}
