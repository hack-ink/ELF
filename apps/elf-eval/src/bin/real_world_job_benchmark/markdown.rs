use std::path::Path;

use super::{
	formatting::{
		adapter_status_str, round3, scenario_comparison_outcome_str, status_str,
		trace_failure_stage,
	},
	*,
};

#[path = "markdown/adapters.rs"] mod adapters;
#[path = "markdown/common.rs"] mod common;
#[path = "markdown/domain_metrics.rs"] mod domain_metrics;
#[path = "markdown/evolution.rs"] mod evolution;
#[path = "markdown/followups.rs"] mod followups;
#[path = "markdown/header.rs"] mod header;
#[path = "markdown/jobs.rs"] mod jobs;
#[path = "markdown/operational.rs"] mod operational;
#[path = "markdown/scoreboard.rs"] mod scoreboard;
#[path = "markdown/trace.rs"] mod trace;

use self::{
	adapters::*, common::*, domain_metrics::*, evolution::*, followups::*, header::*, jobs::*,
	operational::*, scoreboard::*, trace::*,
};

pub(super) fn render_markdown(report: &RealWorldReport, report_path: &Path) -> String {
	let report_path = report_path.display().to_string();
	let mut out = String::new();

	render_markdown_header(&mut out, report, report_path.as_str());
	render_markdown_scoreboard(&mut out, report);
	render_markdown_operational_evidence(&mut out, report);
	render_markdown_external_adapters(&mut out, report);
	render_markdown_capture_integration(&mut out, report);
	render_markdown_suites(&mut out, report);
	render_markdown_jobs(&mut out, report);
	render_markdown_operator_debugging(&mut out, report);
	render_markdown_evolution(&mut out, report);
	render_markdown_trace_explainability(&mut out, report);
	render_markdown_consolidation(&mut out, report);
	render_markdown_memory_summary(&mut out, report);
	render_markdown_proactive_brief(&mut out, report);
	render_markdown_scheduled_memory(&mut out, report);
	render_markdown_work_continuity(&mut out, report);
	render_markdown_knowledge(&mut out, report);
	render_markdown_unsupported_claims(&mut out, report);
	render_markdown_follow_ups(&mut out, report);
	render_markdown_semantics(&mut out, report);

	out
}
