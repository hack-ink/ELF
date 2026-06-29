use super::{
	ConsolidationInputRef, ConsolidationProposalInput, Deserialize, Serialize, serde_json,
};

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct LiveConsolidationFixture {
	#[serde(default)]
	pub(crate) proposals: Vec<LiveConsolidationProposal>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct LiveConsolidationProposal {
	pub(crate) proposal_id: String,
	pub(crate) proposal_kind: String,
	#[serde(default)]
	pub(crate) source_refs: Vec<String>,
	#[serde(default)]
	pub(crate) expected_source_refs: Vec<String>,
	pub(crate) usefulness_score: f64,
	pub(crate) min_usefulness_score: f64,
	pub(crate) expected_review_action: String,
	pub(crate) actual_review_action: String,
	#[serde(default)]
	pub(crate) source_mutations: Vec<serde_json::Value>,
	#[serde(default)]
	pub(crate) unsupported_claim_count: usize,
	#[serde(default)]
	pub(crate) unsupported_claim_flags: Vec<LiveUnsupportedClaimFlag>,
	#[serde(default)]
	pub(crate) diff: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct LiveUnsupportedClaimFlag {
	pub(crate) claim_id: Option<String>,
	pub(crate) message: String,
	pub(crate) source_ref: Option<String>,
}

#[derive(Debug)]
pub(crate) struct PreparedConsolidationRun {
	pub(crate) input_refs: Vec<ConsolidationInputRef>,
	pub(crate) proposals: Vec<ConsolidationProposalInput>,
}
