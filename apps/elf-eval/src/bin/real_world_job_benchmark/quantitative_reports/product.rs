use crate::{Deserialize, QuantitativeBenchmarkRow, QuantitativePerQueryRow, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct QuantitativeProductManifest {
	pub(crate) schema: String,
	pub(crate) manifest_id: String,
	pub(crate) corpus_id: String,
	#[serde(default)]
	pub(crate) rows: Vec<QuantitativeBenchmarkRow>,
	#[serde(default)]
	pub(crate) per_query_rows: Vec<QuantitativePerQueryRow>,
}
