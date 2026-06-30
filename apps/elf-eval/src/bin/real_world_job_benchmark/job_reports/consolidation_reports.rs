use crate::{ConsolidationReviewAction, Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ConsolidationJobReport {
	pub(crate) proposal_count: usize,
	pub(crate) proposal_usefulness: Option<f64>,
	pub(crate) lineage_completeness: Option<f64>,
	pub(crate) review_action_correctness: Option<f64>,
	pub(crate) source_mutation_count: usize,
	pub(crate) proposal_unsupported_claim_count: usize,
	pub(crate) executable_gaps: Vec<ConsolidationExecutableGapReport>,
	pub(crate) proposals: Vec<ConsolidationProposalReport>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ConsolidationProposalReport {
	pub(crate) proposal_id: String,
	pub(crate) proposal_kind: String,
	pub(crate) usefulness_score: f64,
	pub(crate) min_usefulness_score: f64,
	pub(crate) lineage_completeness: f64,
	pub(crate) expected_review_action: ConsolidationReviewAction,
	pub(crate) actual_review_action: ConsolidationReviewAction,
	pub(crate) review_action_correct: bool,
	pub(crate) source_mutation_count: usize,
	pub(crate) unsupported_claim_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ConsolidationExecutableGapReport {
	pub(crate) primitive: String,
	pub(crate) follow_up_issue: String,
	pub(crate) reason: String,
	pub(crate) blocks_fixture_pass: bool,
}
