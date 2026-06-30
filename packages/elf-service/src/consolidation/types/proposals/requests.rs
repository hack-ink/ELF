use serde::Deserialize;
use uuid::Uuid;

use elf_domain::consolidation::{ConsolidationReviewAction, ConsolidationReviewState};

/// Request to get one consolidation proposal.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationProposalGetRequest {
	/// Tenant that owns the proposal.
	pub tenant_id: String,
	/// Project that owns the proposal.
	pub project_id: String,
	/// Proposal identifier.
	pub proposal_id: Uuid,
}

/// Request to list consolidation proposals.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationProposalsListRequest {
	/// Tenant that owns the proposals.
	pub tenant_id: String,
	/// Project that owns the proposals.
	pub project_id: String,
	/// Optional run filter.
	pub run_id: Option<Uuid>,
	/// Optional review-state filter.
	pub review_state: Option<ConsolidationReviewState>,
	/// Maximum number of proposals to return.
	pub limit: Option<u32>,
}

/// Request to apply one proposal review action.
#[derive(Clone, Debug, Deserialize)]
pub struct ConsolidationProposalReviewRequest {
	/// Tenant that owns the proposal.
	pub tenant_id: String,
	/// Project that owns the proposal.
	pub project_id: String,
	/// Agent performing the review action.
	pub reviewer_agent_id: String,
	/// Proposal identifier.
	pub proposal_id: Uuid,
	/// Requested review action.
	pub review_action: ConsolidationReviewAction,
	/// Optional reviewer comment.
	pub review_comment: Option<String>,
}
