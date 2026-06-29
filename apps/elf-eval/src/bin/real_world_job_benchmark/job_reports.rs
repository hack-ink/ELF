use super::*;

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct JobReport {
	pub(super) suite_id: String,
	pub(super) job_id: String,
	pub(super) title: String,
	pub(super) status: TypedStatus,
	pub(super) operational_evidence_tier: String,
	pub(super) answer_type: String,
	pub(super) requires_caveat: bool,
	pub(super) requires_refusal: bool,
	pub(super) can_answer_unknown: bool,
	pub(super) normalized_score: f64,
	pub(super) hard_fail_hits: Vec<String>,
	pub(super) expected_evidence: Vec<ExpectedEvidenceReport>,
	pub(super) produced_answer: String,
	pub(super) produced_evidence: Vec<String>,
	pub(super) unsupported_claim_count: usize,
	pub(super) wrong_result_count: usize,
	#[serde(default)]
	pub(super) stale_answer_count: usize,
	#[serde(default)]
	pub(super) conflict_detection_count: usize,
	#[serde(default)]
	pub(super) update_rationale_available: bool,
	#[serde(default)]
	pub(super) temporal_validity_not_encoded: bool,
	#[serde(default)]
	pub(super) history_readback_encoded: bool,
	pub(super) retrieval_quality: RetrievalQualityReport,
	pub(super) latency_ms: Option<f64>,
	pub(super) cost: Option<CostReport>,
	pub(super) trace_explainability: Option<TraceExplainability>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) knowledge: Option<KnowledgeJobMetrics>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) memory_summary: Option<MemorySummaryJobMetrics>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) proactive_brief: Option<ProactiveBriefJobMetrics>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) scheduled_memory: Option<ScheduledMemoryJobMetrics>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) work_continuity: Option<WorkContinuityJobMetrics>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub(super) recovery_drills: Vec<AuthorityRecoveryDrillArtifact>,
	pub(super) trap_ids_used: Vec<String>,
	pub(super) dimension_scores: Vec<DimensionScoreReport>,
	pub(super) reason: String,
	#[serde(default)]
	pub(super) evidence_required_count: usize,
	#[serde(default)]
	pub(super) evidence_covered_count: usize,
	#[serde(default)]
	pub(super) source_ref_required_count: usize,
	#[serde(default)]
	pub(super) source_ref_covered_count: usize,
	#[serde(default)]
	pub(super) quote_required_count: usize,
	#[serde(default)]
	pub(super) quote_covered_count: usize,
	#[serde(default)]
	pub(super) stale_retrieval_count: usize,
	#[serde(default)]
	pub(super) scope_check_count: usize,
	#[serde(default)]
	pub(super) scope_correct_count: usize,
	#[serde(default)]
	pub(super) scope_violation_count: usize,
	#[serde(default)]
	pub(super) redaction_leak_count: usize,
	#[serde(default)]
	pub(super) qdrant_rebuild_case: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) operator_debug: Option<OperatorDebugEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) evolution: Option<EvolutionJobReport>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) consolidation: Option<ConsolidationJobReport>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct ExpectedEvidenceReport {
	pub(super) evidence_id: String,
	pub(super) claim_id: String,
	pub(super) requirement: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct DimensionScoreReport {
	pub(super) dimension: String,
	pub(super) score: f64,
	pub(super) max_points: f64,
	pub(super) weight: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct RetrievalQualityReport {
	pub(super) expected_evidence_total: usize,
	pub(super) expected_evidence_matched: usize,
	pub(super) expected_evidence_recall: f64,
	pub(super) produced_evidence_total: usize,
	pub(super) irrelevant_context_count: usize,
	pub(super) irrelevant_context_ratio: f64,
	pub(super) trap_context_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ConsolidationJobReport {
	pub(super) proposal_count: usize,
	pub(super) proposal_usefulness: Option<f64>,
	pub(super) lineage_completeness: Option<f64>,
	pub(super) review_action_correctness: Option<f64>,
	pub(super) source_mutation_count: usize,
	pub(super) proposal_unsupported_claim_count: usize,
	pub(super) executable_gaps: Vec<ConsolidationExecutableGapReport>,
	pub(super) proposals: Vec<ConsolidationProposalReport>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ConsolidationProposalReport {
	pub(super) proposal_id: String,
	pub(super) proposal_kind: String,
	pub(super) usefulness_score: f64,
	pub(super) min_usefulness_score: f64,
	pub(super) lineage_completeness: f64,
	pub(super) expected_review_action: ConsolidationReviewAction,
	pub(super) actual_review_action: ConsolidationReviewAction,
	pub(super) review_action_correct: bool,
	pub(super) source_mutation_count: usize,
	pub(super) unsupported_claim_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ConsolidationExecutableGapReport {
	pub(super) primitive: String,
	pub(super) follow_up_issue: String,
	pub(super) reason: String,
	pub(super) blocks_fixture_pass: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct UnsupportedClaimReport {
	pub(super) suite_id: String,
	pub(super) job_id: String,
	pub(super) claim_id: Option<String>,
	pub(super) claim_text: String,
	pub(super) reason: String,
	pub(super) evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct KnowledgeJobMetrics {
	pub(super) page_count: usize,
	pub(super) section_count: usize,
	pub(super) traced_section_count: usize,
	pub(super) flagged_unsupported_section_count: usize,
	pub(super) untraced_section_count: usize,
	pub(super) unsupported_summary_count: usize,
	pub(super) backlink_count: usize,
	pub(super) pages_with_backlinks: usize,
	pub(super) pages_with_version_diff: usize,
	pub(super) stale_trap_count: usize,
	pub(super) stale_traps_detected: usize,
	pub(super) rebuild_page_count: usize,
	pub(super) deterministic_rebuild_count: usize,
	pub(super) rebuild_failure_count: usize,
	pub(super) allowed_variance_count: usize,
	pub(super) citation_coverage: f64,
	pub(super) stale_claim_detection: f64,
	pub(super) rebuild_determinism: f64,
	pub(super) backlink_coverage: f64,
	pub(super) version_diff_coverage: f64,
	pub(super) page_usefulness: f64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct MemorySummaryJobMetrics {
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
pub(super) struct ProactiveBriefJobMetrics {
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
pub(super) struct ScheduledMemoryJobMetrics {
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
pub(super) struct WorkContinuityJobMetrics {
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
pub(super) struct EvolutionSummary {
	pub(super) stale_answer_count: usize,
	pub(super) conflict_detection_count: usize,
	pub(super) update_rationale_available_count: usize,
	pub(super) temporal_validity_not_encoded_count: usize,
	pub(super) history_readback_encoded_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct EvolutionJobReport {
	pub(super) current_evidence: Vec<String>,
	pub(super) historical_evidence: Vec<String>,
	pub(super) tombstone_evidence: Vec<String>,
	pub(super) invalidation_evidence: Vec<String>,
	pub(super) selected_current_evidence: Vec<String>,
	pub(super) selected_historical_evidence: Vec<String>,
	pub(super) selected_rationale_evidence: Vec<String>,
	pub(super) selected_tombstone_evidence: Vec<String>,
	pub(super) selected_invalidation_evidence: Vec<String>,
	pub(super) conflict_candidate_evidence: Vec<String>,
	pub(super) retrieved_but_dropped_evidence: Vec<String>,
	pub(super) selected_but_not_narrated_evidence: Vec<String>,
	pub(super) stale_trap_ids_used: Vec<String>,
	pub(super) stale_answer_count: usize,
	pub(super) conflict_count: usize,
	pub(super) conflict_detection_count: usize,
	pub(super) update_rationale_available: bool,
	pub(super) temporal_validity_required: bool,
	pub(super) temporal_validity_encoded: bool,
	pub(super) temporal_validity_not_encoded: bool,
	pub(super) history_readback_encoded: bool,
	pub(super) history_event_types: Vec<String>,
	pub(super) history_requires_note_version_links: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) follow_up: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct FollowUpReport {
	pub(super) suite_id: String,
	pub(super) job_id: String,
	pub(super) title: String,
	pub(super) reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct PrivateCorpusRedaction {
	pub(super) policy: String,
	pub(super) private_fixture_count: usize,
}

#[derive(Debug)]
pub(super) struct JobScoring {
	pub(super) status: TypedStatus,
	pub(super) normalized_score: f64,
	pub(super) hard_fail_hits: Vec<String>,
	pub(super) unsupported_claims: Vec<UnsupportedClaimReport>,
	pub(super) wrong_result_count: usize,
	pub(super) knowledge: Option<KnowledgeJobMetrics>,
	pub(super) trap_ids_used: Vec<String>,
	pub(super) dimension_scores: Vec<DimensionScoreReport>,
	pub(super) reason: String,
	pub(super) evolution: Option<EvolutionJobReport>,
	pub(super) consolidation: Option<ConsolidationJobReport>,
	pub(super) memory_summary: Option<MemorySummaryJobMetrics>,
	pub(super) proactive_brief: Option<ProactiveBriefJobMetrics>,
	pub(super) scheduled_memory: Option<ScheduledMemoryJobMetrics>,
	pub(super) work_continuity: Option<WorkContinuityJobMetrics>,
}

#[derive(Debug, Default)]
pub(super) struct FailureCounts {
	pub(super) missing_claims: usize,
	pub(super) forbidden_claims: usize,
	pub(super) missing_evidence: usize,
	pub(super) trap_uses: usize,
	pub(super) unsupported_claims: usize,
	pub(super) operator_debug_missing: usize,
	pub(super) operator_debug_raw_sql: usize,
	pub(super) operator_debug_trace_gaps: usize,
	pub(super) operator_debug_repair_unclear: usize,
	pub(super) stale_answers: usize,
	pub(super) conflict_detection_missing: usize,
	pub(super) update_rationale_missing: usize,
	pub(super) latency_violations: usize,
	pub(super) proposal_usefulness_failures: usize,
	pub(super) lineage_failures: usize,
	pub(super) review_action_failures: usize,
	pub(super) source_mutations: usize,
	pub(super) blocking_executable_gaps: usize,
	pub(super) memory_summary_invalid_current_entries: usize,
	pub(super) memory_summary_untraced_entries: usize,
	pub(super) memory_summary_missing_freshness: usize,
	pub(super) memory_summary_missing_rationale: usize,
	pub(super) memory_summary_missing_categories: usize,
	pub(super) memory_summary_unsupported_current_entries: usize,
	pub(super) proactive_brief_invalid_current_suggestions: usize,
	pub(super) proactive_brief_untraced_suggestions: usize,
	pub(super) proactive_brief_missing_freshness: usize,
	pub(super) proactive_brief_missing_action_rationale: usize,
	pub(super) proactive_brief_missing_kinds: usize,
	pub(super) proactive_brief_unsupported_current_suggestions: usize,
	pub(super) proactive_brief_tombstone_violations: usize,
	pub(super) scheduled_memory_invalid_current_outputs: usize,
	pub(super) scheduled_memory_untraced_outputs: usize,
	pub(super) scheduled_memory_missing_freshness: usize,
	pub(super) scheduled_memory_missing_action_rationale: usize,
	pub(super) scheduled_memory_missing_task_kinds: usize,
	pub(super) scheduled_memory_unsupported_current_outputs: usize,
	pub(super) scheduled_memory_tombstone_violations: usize,
	pub(super) scheduled_memory_missing_trace: usize,
	pub(super) work_continuity_reset_resume_missing: usize,
	pub(super) work_continuity_decision_rationale_missing: usize,
	pub(super) work_continuity_rejected_option_unsuppressed: usize,
	pub(super) work_continuity_rejected_option_resurrection: usize,
	pub(super) work_continuity_explicit_next_step_missing: usize,
	pub(super) work_continuity_explicit_next_step_extra: usize,
	pub(super) work_continuity_inferred_step_unlabeled: usize,
	pub(super) work_continuity_inferred_step_as_instruction: usize,
	pub(super) work_continuity_handoff_source_ref_missing: usize,
	pub(super) work_continuity_redaction_missing: usize,
	pub(super) work_continuity_sensitive_marker_persistence: usize,
	pub(super) work_continuity_janitor_false_promotion: usize,
	pub(super) work_continuity_journal_only_authority_claim: usize,
	pub(super) untraced_page_sections: usize,
	pub(super) missed_stale_findings: usize,
	pub(super) rebuild_failures: usize,
	pub(super) page_usefulness_failures: usize,
}

#[derive(Debug, Default)]
pub(super) struct JobMetrics {
	pub(super) evidence_required_count: usize,
	pub(super) evidence_covered_count: usize,
	pub(super) source_ref_required_count: usize,
	pub(super) source_ref_covered_count: usize,
	pub(super) quote_required_count: usize,
	pub(super) quote_covered_count: usize,
	pub(super) stale_retrieval_count: usize,
	pub(super) scope_check_count: usize,
	pub(super) scope_correct_count: usize,
	pub(super) scope_violation_count: usize,
	pub(super) redaction_leak_count: usize,
	pub(super) qdrant_rebuild_case: bool,
}

pub(super) struct ScoreboardRankedMetrics {
	pub(super) relevant_at_k: usize,
	pub(super) precision_denominator_at_k: usize,
	pub(super) reciprocal_rank: f64,
	pub(super) ndcg: f64,
}
