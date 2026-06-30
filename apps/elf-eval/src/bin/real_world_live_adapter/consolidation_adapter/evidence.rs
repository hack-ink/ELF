use crate::{
	ConsolidationInputRef, ConsolidationMaterializationEvidence, ConsolidationProposalResponse,
	LiveConsolidationFixture, Uuid,
};

pub(in crate::consolidation_adapter) fn consolidation_materialization_evidence(
	run_id: Uuid,
	fixture: &LiveConsolidationFixture,
	input_refs: &[ConsolidationInputRef],
	reviewed: &[ConsolidationProposalResponse],
) -> ConsolidationMaterializationEvidence {
	let review_actions = reviewed
		.iter()
		.flat_map(|proposal| proposal.review_events.iter().map(|event| event.action.clone()))
		.collect::<Vec<_>>();
	let final_review_states =
		reviewed.iter().map(|proposal| proposal.review_state.clone()).collect::<Vec<_>>();
	let unsupported_claim_flag_count = fixture
		.proposals
		.iter()
		.map(|proposal| {
			proposal.unsupported_claim_count.max(proposal.unsupported_claim_flags.len())
		})
		.sum();
	let review_event_count =
		reviewed.iter().map(|proposal| proposal.review_events.len()).sum::<usize>();

	ConsolidationMaterializationEvidence {
		run_id: Some(run_id),
		proposal_ids: reviewed.iter().map(|proposal| proposal.proposal_id).collect(),
		source_lineage_count: input_refs.len(),
		unsupported_claim_flag_count,
		review_event_count,
		review_actions,
		final_review_states,
	}
}
