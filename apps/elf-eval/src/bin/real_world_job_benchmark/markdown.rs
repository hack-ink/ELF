mod adapters;
mod common;
mod domain_metrics;
mod evolution;
mod followups;
mod header;
mod jobs;
mod operational;
mod quantitative;
mod scoreboard;
mod trace;

use std::path::Path;

use self::common::{bool_display, cost_display, md_cell, md_inline, md_list, md_url, optional_f64};
use crate::{
	AdapterScenarioJudgment, AdapterSource, AdapterStatusCounts, AdapterSuiteCoverage, CostReport,
	DEFAULT_ADAPTER_BEHAVIOR, EvolutionJobReport, ExternalAdapterReport, KnowledgeSummary,
	MemorySummaryReport, OperatorDebugEvidence, OperatorUxGap, ProactiveBriefSummaryReport,
	QuantitativeBenchmarkRow, RealWorldReport, ReportSummary, SCOREBOARD_EVIDENCE_CLASSES,
	ScenarioOutcomeCounts, ScenarioPositionCounts, ScheduledMemorySummaryReport, ScoreboardReport,
	ScoreboardRow, TraceExplainability, WorkContinuitySummaryReport,
	formatting::{
		adapter_status_str, round3, scenario_comparison_outcome_str, status_str,
		trace_failure_stage,
	},
	scenario_comparison_outcome,
};

pub(super) fn render_markdown(report: &RealWorldReport, report_path: &Path) -> String {
	let report_path = report_path.display().to_string();
	let mut out = String::new();

	self::header::render_markdown_header(&mut out, report, report_path.as_str());
	self::scoreboard::render_markdown_scoreboard(&mut out, report);
	self::quantitative::render_markdown_quantitative_scoreboard(&mut out, report);
	self::operational::render_markdown_operational_evidence(&mut out, report);
	self::adapters::render_markdown_external_adapters(&mut out, report);
	self::adapters::render_markdown_capture_integration(&mut out, report);
	self::jobs::render_markdown_suites(&mut out, report);
	self::jobs::render_markdown_jobs(&mut out, report);
	self::jobs::render_markdown_operator_debugging(&mut out, report);
	self::evolution::render_markdown_evolution(&mut out, report);
	self::trace::render_markdown_trace_explainability(&mut out, report);
	self::domain_metrics::render_markdown_consolidation(&mut out, report);
	self::domain_metrics::render_markdown_memory_summary(&mut out, report);
	self::domain_metrics::render_markdown_proactive_brief(&mut out, report);
	self::domain_metrics::render_markdown_scheduled_memory(&mut out, report);
	self::domain_metrics::render_markdown_work_continuity(&mut out, report);
	self::domain_metrics::render_markdown_knowledge(&mut out, report);
	self::followups::render_markdown_unsupported_claims(&mut out, report);
	self::followups::render_markdown_follow_ups(&mut out, report);
	self::followups::render_markdown_semantics(&mut out, report);

	out
}
