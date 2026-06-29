use super::super::*;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ConsolidationFixture {
	#[serde(default)]
	pub(crate) proposals: Vec<ConsolidationProposalFixture>,
	#[serde(default)]
	pub(crate) executable_gaps: Vec<ConsolidationExecutableGap>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ConsolidationProposalFixture {
	pub(crate) proposal_id: String,
	pub(crate) proposal_kind: String,
	#[serde(default)]
	pub(crate) source_refs: Vec<String>,
	#[serde(default)]
	pub(crate) expected_source_refs: Vec<String>,
	pub(crate) usefulness_score: f64,
	pub(crate) min_usefulness_score: f64,
	pub(crate) expected_review_action: ConsolidationReviewAction,
	pub(crate) actual_review_action: ConsolidationReviewAction,
	#[serde(default)]
	pub(crate) source_mutations: Vec<Value>,
	#[serde(default)]
	pub(crate) unsupported_claim_count: usize,
	#[serde(default)]
	pub(crate) unsupported_claim_flags: Vec<Value>,
	#[serde(default)]
	pub(crate) diff: Value,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ConsolidationExecutableGap {
	pub(crate) primitive: String,
	pub(crate) follow_up_issue: String,
	pub(crate) reason: String,
	#[serde(default)]
	pub(crate) blocks_fixture_pass: bool,
}
