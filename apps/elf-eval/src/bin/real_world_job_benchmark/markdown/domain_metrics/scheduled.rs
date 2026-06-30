use crate::markdown::{self, RealWorldReport};

pub(in crate::markdown) fn render_markdown_scheduled_memory(
	out: &mut String,
	report: &RealWorldReport,
) {
	let scheduled_jobs =
		report.jobs.iter().filter(|job| job.scheduled_memory.is_some()).collect::<Vec<_>>();

	if scheduled_jobs.is_empty() {
		return;
	}

	out.push_str("## Scheduled Memory Metrics\n\n");
	out.push_str("| Job | Task Runs | Outputs | Kinds | Evidence Coverage | Freshness | Action Rationale | Trace Coverage | Invalid Current | Untraced | Unsupported Current | Tombstone Violations | Source Mutations |\n");
	out.push_str(
		"| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
	);

	for job in scheduled_jobs {
		let Some(metrics) = &job.scheduled_memory else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | {} | `{}/{}` | `{:.3}` | `{:.3}` | `{:.3}` | `{:.3}` | {} | {} | {} | {} | {} |\n",
			markdown::md_cell(job.job_id.as_str()),
			metrics.task_run_count,
			metrics.output_count,
			metrics.covered_required_task_kind_count,
			metrics.required_task_kind_count,
			metrics.evidence_ref_coverage,
			metrics.freshness_coverage,
			metrics.action_rationale_coverage,
			metrics.trace_coverage,
			metrics.invalid_current_output_count,
			metrics.untraced_output_count,
			metrics.unsupported_current_output_count,
			metrics.tombstone_violation_count,
			metrics.source_mutation_count
		));
	}

	out.push('\n');
}
