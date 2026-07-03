use crate::{CostReport, Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct SourceBackedQualityReport {
	pub(crate) schema: String,
	pub(crate) metric_basis: String,
	pub(crate) result_state: String,
	pub(crate) hard_fail_passed: bool,
	pub(crate) hard_failures: Vec<String>,
	pub(crate) required_metric_names: Vec<String>,
	pub(crate) metrics: SourceBackedQualityMetrics,
	pub(crate) context_pack_decisions: SourceBackedContextPackDecisionCounts,
	pub(crate) scenario_coverage: Vec<SourceBackedScenarioCoverage>,
	pub(crate) typed_result_states_present: Vec<String>,
	pub(crate) artifact_policy: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct SourceBackedQualityMetrics {
	pub(crate) expected_evidence_recall: f64,
	pub(crate) precision_at_5: Option<f64>,
	pub(crate) irrelevant_context_ratio: f64,
	pub(crate) source_ref_coverage: f64,
	pub(crate) stale_suppression_rate: Option<f64>,
	pub(crate) correction_persistence_rate: Option<f64>,
	pub(crate) delete_tombstone_suppression_rate: Option<f64>,
	pub(crate) unsupported_claim_rate: f64,
	pub(crate) cross_scope_leak_count: usize,
	pub(crate) journal_only_authority_claim_count: usize,
	pub(crate) context_pack_activation_precision: Option<f64>,
	pub(crate) context_pack_activation_recall: Option<f64>,
	pub(crate) activation_trace_coverage: Option<f64>,
	pub(crate) mean_latency_ms: Option<f64>,
	pub(crate) total_cost: Option<CostReport>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct SourceBackedContextPackDecisionCounts {
	pub(crate) total_decisions: usize,
	pub(crate) traced_decisions: usize,
	pub(crate) incorrect_decisions: usize,
	pub(crate) expected_enabled: usize,
	pub(crate) observed_enabled: usize,
	pub(crate) correct_enabled: usize,
	pub(crate) expected_suppressed: usize,
	pub(crate) correct_suppressed: usize,
	pub(crate) expected_disabled: usize,
	pub(crate) correct_disabled: usize,
	pub(crate) expected_stale_suppressed: usize,
	pub(crate) correct_stale_suppressed: usize,
	pub(crate) expected_blocked: usize,
	pub(crate) correct_blocked: usize,
	pub(crate) expected_pinned_ineligible: usize,
	pub(crate) correct_pinned_ineligible: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct SourceBackedScenarioCoverage {
	pub(crate) scenario: String,
	pub(crate) status: String,
	pub(crate) covered_job_count: usize,
	pub(crate) pass_count: usize,
	pub(crate) required: bool,
}
