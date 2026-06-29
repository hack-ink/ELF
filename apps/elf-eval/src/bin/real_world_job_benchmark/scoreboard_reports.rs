use super::*;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ScoreboardReport {
	pub(super) schema: String,
	pub(super) result_states: Vec<String>,
	pub(super) evidence_classes: Vec<String>,
	pub(super) metric_basis: String,
	pub(super) retrieval_k: usize,
	pub(super) job_typed_non_pass_count: usize,
	pub(super) job_typed_non_pass_states_present: Vec<String>,
	pub(super) job_summary_claim: String,
	pub(super) external_adapter_typed_non_pass_count: usize,
	pub(super) external_adapter_typed_non_pass_states_present: Vec<String>,
	pub(super) typed_non_pass_count: usize,
	pub(super) typed_non_pass_states_present: Vec<String>,
	pub(super) evidence_class_counts: BTreeMap<String, usize>,
	pub(super) summary_claim: String,
	pub(super) unqualified_win_claim_allowed: bool,
	pub(super) claim_boundary: String,
	#[serde(default)]
	pub(super) rows: Vec<ScoreboardRow>,
	#[serde(default)]
	pub(super) optimization_roadmap: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ScoreboardRow {
	pub(super) product_id: String,
	pub(super) product_name: String,
	pub(super) row_source: String,
	pub(super) evidence_class: String,
	pub(super) result_state: String,
	pub(super) comparable: bool,
	pub(super) same_corpus: bool,
	pub(super) source_id_mapped: bool,
	pub(super) held_out: bool,
	pub(super) leakage_audited: bool,
	pub(super) product_runtime: bool,
	pub(super) container_digest_identified: bool,
	pub(super) metrics: ScoreboardMetrics,
	#[serde(default)]
	pub(super) strengths: Vec<String>,
	#[serde(default)]
	pub(super) weaknesses: Vec<String>,
	#[serde(default)]
	pub(super) next_evidence: Vec<String>,
	#[serde(default)]
	pub(super) source_provenance: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ScoreboardMetrics {
	pub(super) retrieval: ScoreboardRetrievalMetrics,
	pub(super) lifecycle: ScoreboardLifecycleMetrics,
	pub(super) answer_safety: ScoreboardAnswerSafetyMetrics,
	pub(super) operations: ScoreboardOperationalMetrics,
	pub(super) coverage: ScoreboardCoverageMetrics,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ScoreboardRetrievalMetrics {
	pub(super) k: usize,
	pub(super) metric_basis: String,
	pub(super) recall_at_k: Option<f64>,
	pub(super) precision_at_k: Option<f64>,
	pub(super) mrr: Option<f64>,
	pub(super) ndcg: Option<f64>,
	pub(super) expected_evidence_recall: Option<f64>,
	pub(super) citation_source_ref_coverage: Option<f64>,
	pub(super) expected_evidence_matched: usize,
	pub(super) expected_evidence_total: usize,
	pub(super) produced_evidence_total: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ScoreboardLifecycleMetrics {
	pub(super) stale_suppression: Option<f64>,
	pub(super) stale_suppressed_count: usize,
	pub(super) stale_check_count: usize,
	pub(super) update_correctness: Option<f64>,
	pub(super) update_correct_count: usize,
	pub(super) update_check_count: usize,
	pub(super) delete_correctness: Option<f64>,
	pub(super) delete_correct_count: usize,
	pub(super) delete_check_count: usize,
	pub(super) rollback_history_readback_rate: Option<f64>,
	pub(super) rollback_history_readback_count: usize,
	pub(super) rollback_history_check_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ScoreboardAnswerSafetyMetrics {
	pub(super) unsupported_claim_rate: Option<f64>,
	pub(super) unsupported_claim_count: usize,
	pub(super) stale_answer_rate: Option<f64>,
	pub(super) stale_answer_count: usize,
	pub(super) hallucinated_evidence_rate: Option<f64>,
	pub(super) redaction_leak_count: usize,
	pub(super) irrelevant_context_ratio: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ScoreboardOperationalMetrics {
	pub(super) mean_latency_ms: Option<f64>,
	pub(super) total_cost: Option<CostReport>,
	pub(super) resource_envelope_status: String,
	pub(super) resource_envelope_job_count: usize,
	pub(super) resource_envelope_pass_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ScoreboardCoverageMetrics {
	pub(super) job_count: usize,
	pub(super) encoded_suite_count: usize,
	pub(super) pass_count: usize,
	pub(super) typed_non_pass_count: usize,
	pub(super) source_ref_coverage: Option<f64>,
	pub(super) evidence_coverage: Option<f64>,
	pub(super) evidence_class: String,
}
