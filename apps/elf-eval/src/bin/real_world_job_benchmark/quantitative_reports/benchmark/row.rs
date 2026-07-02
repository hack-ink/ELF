use crate::{BTreeMap, Deserialize, QuantitativeConfidenceInterval, Serialize};

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
