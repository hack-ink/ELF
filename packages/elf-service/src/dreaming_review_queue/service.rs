use crate::{
	ConsolidationProposalResponse, ConsolidationProposalReviewEventResponse, ElfService, Result,
	dreaming_review_queue::{
		policy::{self, ELF_DREAMING_REVIEW_QUEUE_SCHEMA_V1},
		types::{
			DreamingReviewQueueItem, DreamingReviewQueuePolicy, DreamingReviewQueueRequest,
			DreamingReviewQueueResponse,
		},
	},
};
use elf_domain::consolidation::ConsolidationReviewState;
use elf_storage::consolidation;

impl ElfService {
	/// Lists consolidation proposals as a Dreaming review queue.
	pub async fn dreaming_review_queue(
		&self,
		req: DreamingReviewQueueRequest,
	) -> Result<DreamingReviewQueueResponse> {
		let limit = policy::bounded_queue_limit(req.limit);
		let review_state = req.review_state.map(ConsolidationReviewState::as_str);
		let proposals = consolidation::list_consolidation_proposals(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.run_id,
			review_state,
			limit,
		)
		.await?;
		let mut items = Vec::with_capacity(proposals.len());

		for proposal in proposals {
			let review_events = consolidation::list_consolidation_proposal_review_events(
				&self.db.pool,
				req.tenant_id.as_str(),
				req.project_id.as_str(),
				proposal.proposal_id,
			)
			.await?
			.into_iter()
			.map(ConsolidationProposalReviewEventResponse::from)
			.collect();
			let mut response = ConsolidationProposalResponse::from(proposal);

			response.review_events = review_events;

			items.push(DreamingReviewQueueItem::from(response));
		}

		Ok(DreamingReviewQueueResponse {
			schema: ELF_DREAMING_REVIEW_QUEUE_SCHEMA_V1.to_string(),
			policy: DreamingReviewQueuePolicy::default(),
			summary: policy::summarize_items(&items),
			items,
		})
	}
}
