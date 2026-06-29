use super::*;

pub(super) fn validate_consolidation_fixture(job: &RealWorldJob, path: &Path) -> Result<()> {
	let consolidation =
		job.corpus.adapter_response.as_ref().and_then(|response| response.consolidation.as_ref());

	if job.suite == "consolidation" && consolidation.is_none() && job.encoding.status.is_none() {
		return Err(eyre::eyre!(
			"{} consolidation jobs must provide adapter_response.consolidation.",
			path.display()
		));
	}

	let Some(consolidation) = consolidation else {
		return Ok(());
	};

	if consolidation.proposals.is_empty() && consolidation.executable_gaps.is_empty() {
		return Err(eyre::eyre!(
			"{} consolidation fixture must provide proposals or executable_gaps.",
			path.display()
		));
	}

	for proposal in &consolidation.proposals {
		validate_consolidation_proposal(proposal, path)?;
	}
	for gap in &consolidation.executable_gaps {
		if gap.primitive.trim().is_empty()
			|| gap.follow_up_issue.trim().is_empty()
			|| gap.reason.trim().is_empty()
		{
			return Err(eyre::eyre!(
				"{} has an incomplete consolidation executable gap.",
				path.display()
			));
		}
	}

	Ok(())
}

fn validate_consolidation_proposal(
	proposal: &ConsolidationProposalFixture,
	path: &Path,
) -> Result<()> {
	if proposal.proposal_id.trim().is_empty()
		|| proposal.proposal_kind.trim().is_empty()
		|| proposal.source_refs.is_empty()
		|| proposal.expected_source_refs.is_empty()
	{
		return Err(eyre::eyre!(
			"{} has an incomplete consolidation proposal fixture.",
			path.display()
		));
	}
	if !proposal.usefulness_score.is_finite()
		|| !proposal.min_usefulness_score.is_finite()
		|| !(0.0..=1.0).contains(&proposal.usefulness_score)
		|| !(0.0..=1.0).contains(&proposal.min_usefulness_score)
	{
		return Err(eyre::eyre!(
			"{} has invalid consolidation proposal usefulness scores.",
			path.display()
		));
	}
	if !proposal.diff.is_null() && !proposal.diff.is_object() {
		return Err(eyre::eyre!(
			"{} consolidation proposal diff must be a JSON object when present.",
			path.display()
		));
	}
	if proposal.unsupported_claim_flags.iter().any(|flag| !flag.is_object()) {
		return Err(eyre::eyre!(
			"{} consolidation unsupported-claim flags must be JSON objects.",
			path.display()
		));
	}

	Ok(())
}
