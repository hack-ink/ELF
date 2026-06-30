use crate::{CostReport, Deserialize, Serialize};

use super::{
	KnowledgeSummary, MemorySummaryReport, ProactiveBriefSummaryReport,
	ScheduledMemorySummaryReport, WorkContinuitySummaryReport,
};

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct ReportSummary {
	pub(crate) job_count: usize,
	pub(crate) encoded_suite_count: usize,
	pub(crate) pass: usize,
	pub(crate) wrong_result: usize,
	pub(crate) lifecycle_fail: usize,
	pub(crate) incomplete: usize,
	pub(crate) blocked: usize,
	pub(crate) not_encoded: usize,
	pub(crate) unsupported_claim: usize,
	pub(crate) unsupported_claim_count: usize,
	pub(crate) wrong_result_count: usize,
	#[serde(default)]
	pub(crate) stale_answer_count: usize,
	#[serde(default)]
	pub(crate) conflict_detection_count: usize,
	#[serde(default)]
	pub(crate) update_rationale_available_count: usize,
	#[serde(default)]
	pub(crate) temporal_validity_not_encoded_count: usize,
	#[serde(default)]
	pub(crate) history_readback_encoded_count: usize,
	pub(crate) expected_evidence_total: usize,
	pub(crate) expected_evidence_matched: usize,
	pub(crate) expected_evidence_recall: f64,
	pub(crate) irrelevant_context_count: usize,
	pub(crate) irrelevant_context_ratio: f64,
	pub(crate) trace_explainability_count: usize,
	pub(crate) wrong_result_stage_attribution_count: usize,
	pub(crate) mean_score: f64,
	pub(crate) mean_latency_ms: Option<f64>,
	pub(crate) total_cost: Option<CostReport>,
	#[serde(default)]
	pub(crate) evidence_required_count: usize,
	#[serde(default)]
	pub(crate) evidence_covered_count: usize,
	#[serde(default)]
	pub(crate) evidence_coverage: f64,
	#[serde(default)]
	pub(crate) source_ref_required_count: usize,
	#[serde(default)]
	pub(crate) source_ref_covered_count: usize,
	#[serde(default)]
	pub(crate) source_ref_coverage: f64,
	#[serde(default)]
	pub(crate) quote_required_count: usize,
	#[serde(default)]
	pub(crate) quote_covered_count: usize,
	#[serde(default)]
	pub(crate) quote_coverage: f64,
	#[serde(default)]
	pub(crate) stale_retrieval_count: usize,
	#[serde(default)]
	pub(crate) scope_check_count: usize,
	#[serde(default)]
	pub(crate) scope_correct_count: usize,
	#[serde(default)]
	pub(crate) scope_correctness: f64,
	#[serde(default)]
	pub(crate) scope_violation_count: usize,
	#[serde(default)]
	pub(crate) redaction_leak_count: usize,
	#[serde(default)]
	pub(crate) qdrant_rebuild_case_count: usize,
	#[serde(default)]
	pub(crate) qdrant_rebuild_pass_count: usize,
	#[serde(default)]
	pub(crate) operator_debug_job_count: usize,
	#[serde(default)]
	pub(crate) raw_sql_needed_count: usize,
	#[serde(default)]
	pub(crate) trace_incomplete_count: usize,
	#[serde(default)]
	pub(crate) operator_ux_gap_count: usize,
	#[serde(default)]
	pub(crate) consolidation: ConsolidationSummaryReport,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) memory_summary: Option<MemorySummaryReport>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) proactive_brief: Option<ProactiveBriefSummaryReport>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) scheduled_memory: Option<ScheduledMemorySummaryReport>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) work_continuity: Option<WorkContinuitySummaryReport>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) knowledge: Option<KnowledgeSummary>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct ConsolidationSummaryReport {
	pub(crate) proposal_count: usize,
	pub(crate) proposal_usefulness: Option<f64>,
	pub(crate) lineage_completeness: Option<f64>,
	pub(crate) review_action_correctness: Option<f64>,
	pub(crate) source_mutation_count: usize,
	pub(crate) proposal_unsupported_claim_count: usize,
	pub(crate) executable_gap_count: usize,
}
