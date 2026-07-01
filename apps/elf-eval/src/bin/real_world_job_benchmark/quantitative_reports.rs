use crate::{BTreeMap, Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct QuantitativeBenchmarkReport {
	pub(super) schema: String,
	pub(super) generated_at: String,
	pub(super) corpus_id: String,
	pub(super) k_values: Vec<usize>,
	pub(super) rows: Vec<QuantitativeBenchmarkRow>,
	#[serde(default)]
	pub(super) per_query_rows: Vec<QuantitativePerQueryRow>,
	#[serde(default)]
	pub(super) metrics_not_encoded: Vec<String>,
	pub(super) controls: QuantitativeBenchmarkControls,
	pub(super) claim_boundary: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct QuantitativeBenchmarkRow {
	pub(super) product: String,
	pub(super) adapter_id: String,
	pub(super) adapter_name: String,
	pub(super) suite: String,
	pub(super) evidence_class: String,
	pub(super) source_manifest_corpus_id: Option<String>,
	pub(super) result_state: String,
	pub(super) comparable: bool,
	pub(super) metric_comparable: bool,
	pub(super) leaderboard_eligible: bool,
	pub(super) held_out: bool,
	pub(super) leakage_audited: bool,
	pub(super) fixture_regression_only: bool,
	pub(super) sample_size: usize,
	pub(super) ranking_query_count: usize,
	pub(super) ranking_coverage_state: String,
	pub(super) ranked_candidate_source: String,
	pub(super) qrel_source: String,
	pub(super) explicit_qrel_query_count: usize,
	pub(super) metrics: BTreeMap<String, Option<f64>>,
	pub(super) metric_states: BTreeMap<String, String>,
	pub(super) denominators: BTreeMap<String, usize>,
	pub(super) claim_boundary: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct QuantitativePerQueryRow {
	pub(super) job_id: String,
	pub(super) suite: String,
	pub(super) evidence_class: String,
	pub(super) source_manifest_corpus_id: Option<String>,
	pub(super) result_state: String,
	pub(super) expected_relevant_count: usize,
	pub(super) candidate_count: usize,
	pub(super) qrel_source: String,
	pub(super) relevance_grade_sum: f64,
	pub(super) product: String,
	pub(super) adapter_id: String,
	pub(super) metrics: BTreeMap<String, Option<f64>>,
	pub(super) metric_states: BTreeMap<String, String>,
	pub(super) denominators: BTreeMap<String, usize>,
	pub(super) claim_boundary: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct QuantitativeBenchmarkControls {
	pub(super) same_corpus_required: bool,
	pub(super) same_task_required: bool,
	pub(super) ranked_candidates_required_for_ranking_metrics: bool,
	pub(super) explicit_relevance_judgments_required_for_leaderboard: bool,
	pub(super) minimum_query_count_for_leaderboard: usize,
	pub(super) current_query_count: usize,
	pub(super) current_ranking_query_count: usize,
	pub(super) current_explicit_qrel_query_count: usize,
	pub(super) leaderboard_claim_allowed: bool,
	pub(super) leakage_control: String,
}
