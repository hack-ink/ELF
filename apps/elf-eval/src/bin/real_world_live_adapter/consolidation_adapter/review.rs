use crate::{
	ConsolidationProposalResponse, ConsolidationReviewAction, LiveConsolidationFixture, LoadedJob,
	Result, eyre,
	serde_json::{self, Value},
};

pub(in crate::consolidation_adapter) fn validate_reviewed_consolidation_count(
	loaded: &LoadedJob,
	fixture: &LiveConsolidationFixture,
	reviewed: &[ConsolidationProposalResponse],
) -> Result<()> {
	if reviewed.len() == fixture.proposals.len() {
		return Ok(());
	}

	Err(eyre::eyre!(
		"ELF consolidation materialized {} proposals for {} fixture proposals in {}.",
		reviewed.len(),
		fixture.proposals.len(),
		loaded.job.job_id
	))
}

pub(in crate::consolidation_adapter) fn consolidation_review_action(
	raw: &str,
) -> Result<ConsolidationReviewAction> {
	match raw {
		"apply" => Ok(ConsolidationReviewAction::Apply),
		"discard" => Ok(ConsolidationReviewAction::Discard),
		"defer" => Ok(ConsolidationReviewAction::Defer),
		"approve" => Ok(ConsolidationReviewAction::Approve),
		_ => Err(eyre::eyre!("Unknown consolidation review action {raw}.")),
	}
}

pub(in crate::consolidation_adapter) fn live_consolidation_response(
	fixture: &LiveConsolidationFixture,
	reviewed: &[ConsolidationProposalResponse],
) -> Result<Value> {
	let proposals = fixture
		.proposals
		.iter()
		.zip(reviewed)
		.map(|(fixture_proposal, reviewed_proposal)| {
			serde_json::json!({
				"proposal_id": reviewed_proposal.proposal_id.to_string(),
				"proposal_kind": fixture_proposal.proposal_kind.clone(),
				"source_refs": fixture_proposal.source_refs.clone(),
				"expected_source_refs": if fixture_proposal.expected_source_refs.is_empty() {
					fixture_proposal.source_refs.clone()
				} else {
					fixture_proposal.expected_source_refs.clone()
				},
				"usefulness_score": fixture_proposal.usefulness_score,
				"min_usefulness_score": fixture_proposal.min_usefulness_score,
				"expected_review_action": fixture_proposal.expected_review_action.clone(),
				"actual_review_action": fixture_proposal.actual_review_action.clone(),
				"source_mutations": fixture_proposal.source_mutations.clone(),
				"unsupported_claim_count": fixture_proposal
					.unsupported_claim_count
					.max(fixture_proposal.unsupported_claim_flags.len()),
				"unsupported_claim_flags": fixture_proposal.unsupported_claim_flags.clone(),
				"diff": fixture_proposal.diff.clone(),
				"live_review_state": reviewed_proposal.review_state.clone(),
				"live_review_event_count": reviewed_proposal.review_events.len()
			})
		})
		.collect::<Vec<_>>();

	Ok(serde_json::json!({ "proposals": proposals, "executable_gaps": [] }))
}
