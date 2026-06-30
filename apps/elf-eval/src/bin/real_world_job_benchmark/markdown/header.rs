mod optional;
mod quality;

use crate::markdown::{self, RealWorldReport};

pub(super) fn render_markdown_header(
	out: &mut String,
	report: &RealWorldReport,
	report_path: &str,
) {
	out.push_str("# Real-World Job Benchmark Report\n\n");
	out.push_str(
		"Goal: Publish a Markdown summary for one generated real_world_job benchmark report.\n",
	);
	out.push_str(
		"Read this when: You need a durable smoke report for real-world agent memory job fixtures.\n",
	);
	out.push_str(&format!("Inputs: `{}`.\n", markdown::md_inline(report_path)));
	out.push_str("Depends on: `apps/elf-eval/fixtures/`, `docs/spec/real_world_agent_memory_benchmark_v1.md`, and `Makefile.toml`.\n");
	out.push_str(
		"Verification: Compare this Markdown summary with the source JSON before committing.\n\n",
	);
	out.push_str("## Summary\n\n");
	out.push_str(&format!("- Run ID: `{}`\n", markdown::md_inline(report.run_id.as_str())));
	out.push_str(&format!(
		"- Generated at: `{}`\n",
		markdown::md_inline(report.generated_at.as_str())
	));
	out.push_str(&format!(
		"- Runner version: `{}`\n",
		markdown::md_inline(report.runner_version.as_str())
	));
	out.push_str(&format!(
		"- Corpus profile: `{}`\n",
		markdown::md_inline(report.corpus_profile.as_str())
	));
	out.push_str(&format!(
		"- Adapter: `{}` ({})\n",
		markdown::md_inline(report.adapter.adapter_id.as_str()),
		markdown::md_inline(report.adapter.behavior.as_str())
	));
	out.push_str(&format!("- Jobs: `{}`\n", report.summary.job_count));
	out.push_str(&format!(
		"- Suites with encoded jobs: `{}`\n",
		report.summary.encoded_suite_count
	));
	out.push_str(&format!(
		"- Suites with `not_encoded` status: `{}`\n",
		report.not_encoded_suites.len()
	));
	out.push_str(&format!("- Status summary: `{}` pass, `{}` wrong_result, `{}` lifecycle_fail, `{}` incomplete, `{}` blocked, `{}` not_encoded, `{}` unsupported_claim\n", report.summary.pass, report.summary.wrong_result, report.summary.lifecycle_fail, report.summary.incomplete, report.summary.blocked, report.summary.not_encoded, report.summary.unsupported_claim));
	out.push_str(&format!(
		"- Unsupported claim count: `{}`\n",
		report.summary.unsupported_claim_count
	));
	out.push_str(&format!("- Wrong-result count: `{}`\n", report.summary.wrong_result_count));
	out.push_str(&format!("- Stale-answer count: `{}`\n", report.summary.stale_answer_count));
	out.push_str(&format!(
		"- Conflict detections: `{}`\n",
		report.summary.conflict_detection_count
	));
	out.push_str(&format!(
		"- Update rationales available: `{}`\n",
		report.summary.update_rationale_available_count
	));
	out.push_str(&format!(
		"- Temporal validity not encoded: `{}`\n",
		report.summary.temporal_validity_not_encoded_count
	));
	out.push_str(&format!(
		"- History readback encoded: `{}`\n",
		report.summary.history_readback_encoded_count
	));

	quality::render_markdown_quality_summary(out, report);

	out.push_str(&format!("- Mean score: `{:.3}`\n", report.summary.mean_score));
	out.push_str(&format!(
		"- Mean latency: `{}`\n",
		markdown::optional_f64(report.summary.mean_latency_ms, " ms")
	));
	out.push_str(&format!(
		"- Cost: `{}`\n",
		markdown::cost_display(report.summary.total_cost.as_ref())
	));
	out.push_str(&format!(
		"- Operator-debug jobs: `{}`\n",
		report.summary.operator_debug_job_count
	));
	out.push_str(&format!("- Raw SQL needed: `{}`\n", report.summary.raw_sql_needed_count));
	out.push_str(&format!(
		"- Trace-incomplete debug jobs: `{}`\n",
		report.summary.trace_incomplete_count
	));
	out.push_str(&format!("- Operator UX gaps: `{}`\n", report.summary.operator_ux_gap_count));

	optional::render_markdown_optional_summary_metrics(out, &report.summary);

	out.push_str(&format!(
		"- Private corpus redaction: `{}`\n\n",
		markdown::md_inline(report.private_corpus_redaction.policy.as_str())
	));
}
