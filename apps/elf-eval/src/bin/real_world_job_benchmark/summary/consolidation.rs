use crate::summary::{self, ConsolidationSummaryReport, JobReport};

pub(super) fn consolidation_summary_impl(jobs: &[JobReport]) -> ConsolidationSummaryReport {
	let reports = jobs.iter().filter_map(|job| job.consolidation.as_ref()).collect::<Vec<_>>();

	if reports.is_empty() {
		return ConsolidationSummaryReport::default();
	}

	let proposals = reports.iter().flat_map(|report| report.proposals.iter()).collect::<Vec<_>>();
	let executable_gap_count = reports.iter().map(|report| report.executable_gaps.len()).sum();

	ConsolidationSummaryReport {
		proposal_count: proposals.len(),
		proposal_usefulness: summary::mean_proposal_metric(
			proposals.iter().map(|proposal| proposal.usefulness_score),
		),
		lineage_completeness: summary::mean_proposal_metric(
			proposals.iter().map(|proposal| proposal.lineage_completeness),
		),
		review_action_correctness: summary::mean_proposal_metric(
			proposals.iter().map(|proposal| if proposal.review_action_correct { 1.0 } else { 0.0 }),
		),
		source_mutation_count: proposals
			.iter()
			.map(|proposal| proposal.source_mutation_count)
			.sum(),
		proposal_unsupported_claim_count: proposals
			.iter()
			.map(|proposal| proposal.unsupported_claim_count)
			.sum(),
		executable_gap_count,
	}
}
