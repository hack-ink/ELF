use crate::markdown::{self, RealWorldReport};

pub(in crate::markdown) fn render_markdown_proactive_brief(
	out: &mut String,
	report: &RealWorldReport,
) {
	let proactive_jobs =
		report.jobs.iter().filter(|job| job.proactive_brief.is_some()).collect::<Vec<_>>();

	if proactive_jobs.is_empty() {
		return;
	}

	out.push_str("## Proactive Brief Metrics\n\n");
	out.push_str("| Job | Briefs | Suggestions | Kinds | Evidence Coverage | Freshness | Action Rationale | Invalid Current | Untraced | Unsupported Current | Tombstone Violations | Rejected | Deferred |\n");
	out.push_str(
		"| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
	);

	for job in proactive_jobs {
		let Some(metrics) = &job.proactive_brief else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | {} | `{}/{}` | `{:.3}` | `{:.3}` | `{:.3}` | {} | {} | {} | {} | {} | {} |\n",
			markdown::md_cell(job.job_id.as_str()),
			metrics.brief_count,
			metrics.suggestion_count,
			metrics.covered_required_suggestion_kind_count,
			metrics.required_suggestion_kind_count,
			metrics.evidence_ref_coverage,
			metrics.freshness_coverage,
			metrics.action_rationale_coverage,
			metrics.invalid_current_suggestion_count,
			metrics.untraced_suggestion_count,
			metrics.unsupported_current_suggestion_count,
			metrics.tombstone_violation_count,
			metrics.rejected_count,
			metrics.deferred_count
		));
	}

	out.push('\n');
}
