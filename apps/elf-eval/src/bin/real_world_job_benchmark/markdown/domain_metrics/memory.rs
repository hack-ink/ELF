use crate::markdown::{self, RealWorldReport};

pub(in crate::markdown) fn render_markdown_memory_summary(
	out: &mut String,
	report: &RealWorldReport,
) {
	let memory_jobs =
		report.jobs.iter().filter(|job| job.memory_summary.is_some()).collect::<Vec<_>>();

	if memory_jobs.is_empty() {
		return;
	}

	out.push_str("## Memory Summary Metrics\n\n");
	out.push_str("| Job | Summaries | Entries | Categories | Source Coverage | Freshness | Rationale | Invalid Top-of-Mind | Untraced | Derived Unsupported | Unsupported Current | Tombstone Refs |\n");
	out.push_str(
		"| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
	);

	for job in memory_jobs {
		let Some(metrics) = &job.memory_summary else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | {} | `{}/{}` | `{:.3}` | `{:.3}` | `{:.3}` | {} | {} | {} | {} | {} |\n",
			markdown::md_cell(job.job_id.as_str()),
			metrics.summary_count,
			metrics.entry_count,
			metrics.covered_required_category_count,
			metrics.required_category_count,
			metrics.source_ref_coverage,
			metrics.freshness_coverage,
			metrics.rationale_coverage,
			metrics.invalid_top_of_mind_count,
			metrics.untraced_entry_count,
			metrics.unsupported_derived_entry_count,
			metrics.unsupported_current_entry_count,
			metrics.tombstone_ref_count
		));
	}

	out.push('\n');
}
