use crate::markdown::{self, RealWorldReport};

pub(in crate::markdown) fn render_markdown_consolidation(
	out: &mut String,
	report: &RealWorldReport,
) {
	if report.summary.consolidation.proposal_count == 0 {
		return;
	}

	out.push_str("## Consolidation\n\n");
	out.push_str("| Job | Proposals | Usefulness | Lineage | Review Actions | Source Mutations | Proposal Unsupported Claims | Executable Gaps |\n");
	out.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");

	for job in &report.jobs {
		let Some(consolidation) = &job.consolidation else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | `{}` | `{}` | `{}` | {} | {} | {} |\n",
			markdown::md_cell(job.job_id.as_str()),
			consolidation.proposal_count,
			markdown::optional_f64(consolidation.proposal_usefulness, ""),
			markdown::optional_f64(consolidation.lineage_completeness, ""),
			markdown::optional_f64(consolidation.review_action_correctness, ""),
			consolidation.source_mutation_count,
			consolidation.proposal_unsupported_claim_count,
			consolidation.executable_gaps.len()
		));
	}

	out.push_str(
		"\nSource mutation count must remain `0` for proposal-only consolidation cases.\n\n",
	);

	render_markdown_consolidation_gaps(out, report);
}

fn render_markdown_consolidation_gaps(out: &mut String, report: &RealWorldReport) {
	let gaps = report
		.jobs
		.iter()
		.filter_map(|job| job.consolidation.as_ref().map(|consolidation| (job, consolidation)))
		.flat_map(|(job, consolidation)| {
			consolidation.executable_gaps.iter().map(move |gap| (job.job_id.as_str(), gap))
		})
		.collect::<Vec<_>>();

	if gaps.is_empty() {
		return;
	}

	out.push_str("### Executable Gaps\n\n");
	out.push_str("| Job | Primitive | Follow-Up Issue | Blocks Fixture Pass | Reason |\n");
	out.push_str("| --- | --- | --- | --- | --- |\n");

	for (job_id, gap) in gaps {
		out.push_str(&format!(
			"| {} | {} | {} | `{}` | {} |\n",
			markdown::md_cell(job_id),
			markdown::md_cell(gap.primitive.as_str()),
			markdown::md_cell(gap.follow_up_issue.as_str()),
			gap.blocks_fixture_pass,
			markdown::md_cell(gap.reason.as_str())
		));
	}

	out.push('\n');
}
