use crate::{BTreeMap, Deserialize, Serialize};

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
