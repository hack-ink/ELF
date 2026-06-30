mod input;
mod requests;
mod responses;

pub use self::{
	input::ConsolidationProposalInput,
	requests::{
		ConsolidationProposalGetRequest, ConsolidationProposalReviewRequest,
		ConsolidationProposalsListRequest,
	},
	responses::{
		ConsolidationProposalResponse, ConsolidationProposalReviewEventResponse,
		ConsolidationProposalsListResponse,
	},
};
