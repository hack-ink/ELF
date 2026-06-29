use crate::markdown::{self, RealWorldReport, TraceExplainability};

pub(super) fn render_markdown_trace_explainability(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Trace Explainability\n\n");

	let jobs =
		report.jobs.iter().filter(|job| job.trace_explainability.is_some()).collect::<Vec<_>>();

	if jobs.is_empty() {
		out.push_str("No encoded job reported trace explainability metadata.\n\n");

		return;
	}

	out.push_str("| Suite | Job | Trace | Failure Stage | Reason | Stage Evidence |\n");
	out.push_str("| --- | --- | --- | --- | --- | --- |\n");

	for job in jobs {
		let trace = job.trace_explainability.as_ref();

		out.push_str(&format!(
			"| {} | {} | `{}` | `{}` | {} | {} |\n",
			markdown::md_cell(job.suite_id.as_str()),
			markdown::md_cell(job.job_id.as_str()),
			markdown::md_inline(trace.and_then(|trace| trace.trace_id.as_deref()).unwrap_or("-")),
			markdown::md_inline(markdown::trace_failure_stage(trace).unwrap_or("-")),
			markdown::md_cell(trace_failure_reason(trace).unwrap_or("-")),
			markdown::md_cell(trace_stage_summary(trace).as_str())
		));
	}

	out.push('\n');
}

fn trace_failure_reason(trace: Option<&TraceExplainability>) -> Option<&str> {
	trace.and_then(|trace| trace.failure_reason.as_deref())
}

fn trace_stage_summary(trace: Option<&TraceExplainability>) -> String {
	let Some(trace) = trace else {
		return "-".to_string();
	};
	let stages = trace
		.stages
		.iter()
		.map(|stage| {
			format!(
				"{} kept={} demoted={} dropped={} distractors={}",
				stage.stage_name,
				stage.kept_evidence.join("+"),
				stage.demoted_evidence.join("+"),
				stage.dropped_evidence.join("+"),
				stage.distractor_evidence.join("+")
			)
		})
		.collect::<Vec<_>>();

	if stages.is_empty() { "-".to_string() } else { stages.join("; ") }
}
