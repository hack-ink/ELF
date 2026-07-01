use crate::{BTreeMap, Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct QuantitativeBenchmarkReport {
	pub(crate) schema: String,
	pub(crate) generated_at: String,
	pub(crate) corpus_id: String,
	pub(crate) k_values: Vec<usize>,
	pub(crate) rows: Vec<QuantitativeBenchmarkRow>,
	#[serde(default)]
	pub(crate) per_query_rows: Vec<QuantitativePerQueryRow>,
	#[serde(default)]
	pub(crate) metrics_not_encoded: Vec<String>,
	pub(crate) controls: QuantitativeBenchmarkControls,
	pub(crate) claim_boundary: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct QuantitativeBenchmarkRow {
	pub(crate) product: String,
	pub(crate) adapter_id: String,
	pub(crate) adapter_name: String,
	pub(crate) suite: String,
	pub(crate) evidence_class: String,
	pub(crate) source_manifest_corpus_id: Option<String>,
	pub(crate) result_state: String,
	pub(crate) comparable: bool,
	pub(crate) metric_comparable: bool,
	pub(crate) leaderboard_eligible: bool,
	pub(crate) held_out: bool,
	pub(crate) leakage_audited: bool,
	pub(crate) audit_manifest_id: Option<String>,
	pub(crate) fixture_regression_only: bool,
	pub(crate) sample_size: usize,
	pub(crate) ranking_query_count: usize,
	pub(crate) ranking_coverage_state: String,
	pub(crate) ranked_candidate_source: String,
	pub(crate) qrel_source: String,
	pub(crate) explicit_qrel_query_count: usize,
	pub(crate) metrics: BTreeMap<String, Option<f64>>,
	pub(crate) metric_states: BTreeMap<String, String>,
	pub(crate) denominators: BTreeMap<String, usize>,
	#[serde(default)]
	pub(crate) confidence_intervals: BTreeMap<String, QuantitativeConfidenceInterval>,
	pub(crate) claim_boundary: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct QuantitativePerQueryRow {
	pub(crate) job_id: String,
	pub(crate) suite: String,
	pub(crate) evidence_class: String,
	pub(crate) source_manifest_corpus_id: Option<String>,
	pub(crate) result_state: String,
	pub(crate) expected_relevant_count: usize,
	pub(crate) candidate_count: usize,
	pub(crate) qrel_source: String,
	pub(crate) relevance_grade_sum: f64,
	pub(crate) product: String,
	pub(crate) adapter_id: String,
	pub(crate) metrics: BTreeMap<String, Option<f64>>,
	pub(crate) metric_states: BTreeMap<String, String>,
	pub(crate) denominators: BTreeMap<String, usize>,
	pub(crate) claim_boundary: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct QuantitativeBenchmarkControls {
	pub(crate) same_corpus_required: bool,
	pub(crate) same_task_required: bool,
	pub(crate) ranked_candidates_required_for_ranking_metrics: bool,
	pub(crate) explicit_relevance_judgments_required_for_leaderboard: bool,
	pub(crate) minimum_query_count_for_leaderboard: usize,
	pub(crate) current_query_count: usize,
	pub(crate) current_ranking_query_count: usize,
	pub(crate) current_explicit_qrel_query_count: usize,
	pub(crate) leaderboard_claim_allowed: bool,
	pub(crate) leakage_control: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct QuantitativeConfidenceInterval {
	pub(crate) method: String,
	pub(crate) confidence: f64,
	pub(crate) lower: f64,
	pub(crate) upper: f64,
	pub(crate) numerator: usize,
	pub(crate) denominator: usize,
}
