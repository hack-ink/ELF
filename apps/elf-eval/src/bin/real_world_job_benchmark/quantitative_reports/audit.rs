use crate::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct QuantitativeAuditManifest {
	pub(crate) schema: String,
	pub(crate) manifest_id: String,
	pub(crate) run_id: String,
	pub(crate) corpus_id: String,
	pub(crate) product: String,
	pub(crate) adapter_id: String,
	pub(crate) held_out: bool,
	pub(crate) leakage_audited: bool,
	pub(crate) sample_size: usize,
	pub(crate) ranking_query_count: usize,
	pub(crate) explicit_qrel_query_count: usize,
	pub(crate) query_ids: Vec<String>,
	#[serde(default)]
	pub(crate) controls: Vec<String>,
	#[serde(default)]
	pub(crate) artifacts: Vec<QuantitativeAuditArtifact>,
	pub(crate) claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct QuantitativeAuditArtifact {
	pub(crate) role: String,
	pub(crate) path: String,
	pub(crate) sha256: String,
}
