use crate::{
	ElfService, Error, Result,
	consolidation::{
		service::review,
		types::{
			ConsolidationProposalGetRequest, ConsolidationProposalResponse,
			ConsolidationProposalsListRequest, ConsolidationProposalsListResponse,
		},
		validation,
	},
};
use elf_domain::consolidation::ConsolidationReviewState;
use elf_storage::consolidation;

impl ElfService {
	/// Fetches one consolidation proposal.
	pub async fn consolidation_proposal_get(
		&self,
		req: ConsolidationProposalGetRequest,
	) -> Result<ConsolidationProposalResponse> {
		let proposal = consolidation::get_consolidation_proposal(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.proposal_id,
		)
		.await?
		.ok_or_else(|| Error::NotFound {
			message: "consolidation proposal not found".to_string(),
		})?;
		let review_events = review::proposal_review_events(
			self,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.proposal_id,
		)
		.await?;
		let mut response = ConsolidationProposalResponse::from(proposal);

		response.review_events = review_events;

		Ok(response)
	}

	/// Lists consolidation proposals.
	pub async fn consolidation_proposals_list(
		&self,
		req: ConsolidationProposalsListRequest,
	) -> Result<ConsolidationProposalsListResponse> {
		let limit = validation::bounded_limit(req.limit);
		let review_state = req.review_state.map(ConsolidationReviewState::as_str);
		let rows = consolidation::list_consolidation_proposals(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.run_id,
			review_state,
			limit,
		)
		.await?;
		let proposals = rows.into_iter().map(ConsolidationProposalResponse::from).collect();

		Ok(ConsolidationProposalsListResponse { proposals })
	}
}
