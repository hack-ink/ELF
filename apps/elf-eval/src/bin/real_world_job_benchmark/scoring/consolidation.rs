use super::*;

pub(super) fn consolidation_job_report(job: &RealWorldJob) -> Option<ConsolidationJobReport> {
	let fixture = job.corpus.adapter_response.as_ref()?.consolidation.as_ref()?;
	let proposals = fixture.proposals.iter().map(consolidation_proposal_report).collect::<Vec<_>>();
	let executable_gaps = fixture
		.executable_gaps
		.iter()
		.map(|gap| ConsolidationExecutableGapReport {
			primitive: gap.primitive.clone(),
			follow_up_issue: gap.follow_up_issue.clone(),
			reason: gap.reason.clone(),
			blocks_fixture_pass: gap.blocks_fixture_pass,
		})
		.collect::<Vec<_>>();
	let proposal_count = proposals.len();
	let source_mutation_count =
		proposals.iter().map(|proposal| proposal.source_mutation_count).sum();
	let proposal_unsupported_claim_count =
		proposals.iter().map(|proposal| proposal.unsupported_claim_count).sum();

	Some(ConsolidationJobReport {
		proposal_count,
		proposal_usefulness: mean_proposal_metric(
			proposals.iter().map(|proposal| proposal.usefulness_score),
		),
		lineage_completeness: mean_proposal_metric(
			proposals.iter().map(|proposal| proposal.lineage_completeness),
		),
		review_action_correctness: mean_proposal_metric(
			proposals.iter().map(|proposal| if proposal.review_action_correct { 1.0 } else { 0.0 }),
		),
		source_mutation_count,
		proposal_unsupported_claim_count,
		executable_gaps,
		proposals,
	})
}

fn consolidation_proposal_report(
	proposal: &ConsolidationProposalFixture,
) -> ConsolidationProposalReport {
	ConsolidationProposalReport {
		proposal_id: proposal.proposal_id.clone(),
		proposal_kind: proposal.proposal_kind.clone(),
		usefulness_score: round3(proposal.usefulness_score),
		min_usefulness_score: round3(proposal.min_usefulness_score),
		lineage_completeness: round3(lineage_completeness(proposal)),
		expected_review_action: proposal.expected_review_action,
		actual_review_action: proposal.actual_review_action,
		review_action_correct: proposal.expected_review_action == proposal.actual_review_action,
		source_mutation_count: proposal.source_mutations.len()
			+ forbidden_diff_key_count(&proposal.diff),
		unsupported_claim_count: proposal
			.unsupported_claim_count
			.max(proposal.unsupported_claim_flags.len()),
	}
}

fn lineage_completeness(proposal: &ConsolidationProposalFixture) -> f64 {
	let expected = proposal.expected_source_refs.iter().collect::<BTreeSet<_>>();
	let actual = proposal.source_refs.iter().collect::<BTreeSet<_>>();
	let matched = expected.iter().filter(|source_ref| actual.contains(**source_ref)).count();

	matched as f64 / expected.len() as f64
}

pub(super) fn proposal_usefulness_failures(
	consolidation: Option<&ConsolidationJobReport>,
) -> usize {
	consolidation.map_or(0, |report| {
		report
			.proposals
			.iter()
			.filter(|proposal| proposal.usefulness_score < proposal.min_usefulness_score)
			.count()
	})
}

pub(super) fn lineage_failures(consolidation: Option<&ConsolidationJobReport>) -> usize {
	consolidation.map_or(0, |report| {
		report.proposals.iter().filter(|proposal| proposal.lineage_completeness < 1.0).count()
	})
}

pub(super) fn review_action_failures(consolidation: Option<&ConsolidationJobReport>) -> usize {
	consolidation.map_or(0, |report| {
		report.proposals.iter().filter(|proposal| !proposal.review_action_correct).count()
	})
}

pub(super) fn blocking_executable_gaps(consolidation: Option<&ConsolidationJobReport>) -> usize {
	consolidation.map_or(0, |report| {
		report.executable_gaps.iter().filter(|gap| gap.blocks_fixture_pass).count()
	})
}
