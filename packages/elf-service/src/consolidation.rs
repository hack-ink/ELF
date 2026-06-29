//! Fixture-driven consolidation run and proposal service APIs.

mod promotion;
mod service;
mod types;
mod validation;

pub use types::{
	ConsolidationProposalGetRequest, ConsolidationProposalInput, ConsolidationProposalResponse,
	ConsolidationProposalReviewEventResponse, ConsolidationProposalReviewRequest,
	ConsolidationProposalsListRequest, ConsolidationProposalsListResponse,
	ConsolidationRunCreateRequest, ConsolidationRunCreateResponse, ConsolidationRunGetRequest,
	ConsolidationRunResponse, ConsolidationRunsListRequest, ConsolidationRunsListResponse,
};

#[cfg(test)] mod tests;
