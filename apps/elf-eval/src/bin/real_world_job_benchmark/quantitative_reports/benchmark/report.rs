use crate::{
	Deserialize, QuantitativeBenchmarkControls, QuantitativeBenchmarkRow, QuantitativePerQueryRow,
	Serialize,
};

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
