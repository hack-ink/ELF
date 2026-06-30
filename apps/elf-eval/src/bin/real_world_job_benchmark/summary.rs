mod consolidation;
mod knowledge;
mod memory;
mod metrics;
mod proactive;
mod report;
mod scheduled;
mod suites;
mod work;

use crate::{
	ConsolidationSummaryReport, CostReport, EvolutionSummary, FollowUpReport, JobReport,
	KnowledgeSummary, MemorySummaryReport, ProactiveBriefSummaryReport, RealWorldJob,
	ReportSummary, ScheduledMemorySummaryReport, SuiteReport, TypedStatus,
	WorkContinuitySummaryReport,
};

pub(super) fn suite_reports(jobs: &[JobReport]) -> Vec<SuiteReport> {
	suites::suite_reports_impl(jobs)
}

pub(super) fn aggregate_status(jobs: &[&JobReport]) -> TypedStatus {
	suites::aggregate_status_impl(jobs)
}

pub(super) fn report_summary(jobs: &[JobReport], suites: &[SuiteReport]) -> ReportSummary {
	report::report_summary_impl(jobs, suites)
}

pub(super) fn evolution_summary(jobs: &[JobReport]) -> EvolutionSummary {
	report::evolution_summary_impl(jobs)
}

pub(super) fn follow_up_reports(jobs: &[RealWorldJob]) -> Vec<FollowUpReport> {
	report::follow_up_reports_impl(jobs)
}

pub(super) fn ratio(numerator: usize, denominator: usize) -> f64 {
	metrics::ratio_impl(numerator, denominator)
}

pub(super) fn ratio_or(numerator: usize, denominator: usize, empty_value: f64) -> f64 {
	metrics::ratio_or_impl(numerator, denominator, empty_value)
}

pub(super) fn ratio_or_full(numerator: usize, denominator: usize) -> f64 {
	metrics::ratio_or_full_impl(numerator, denominator)
}

pub(super) fn mean_latency_for_reports(jobs: &[&JobReport]) -> Option<f64> {
	metrics::mean_latency_for_reports_impl(jobs)
}

pub(super) fn mean_latency_for_values(latencies: &[f64]) -> Option<f64> {
	metrics::mean_latency_for_values_impl(latencies)
}

pub(super) fn total_cost(jobs: &[JobReport]) -> Option<CostReport> {
	metrics::total_cost_impl(jobs)
}

pub(super) fn total_cost_for_reports(jobs: &[&JobReport]) -> Option<CostReport> {
	metrics::total_cost_for_reports_impl(jobs)
}

pub(super) fn mean_proposal_metric(values: impl Iterator<Item = f64>) -> Option<f64> {
	metrics::mean_proposal_metric_impl(values)
}

fn expected_evidence_recall_for_jobs(jobs: &[&JobReport]) -> f64 {
	metrics::expected_evidence_recall_for_jobs_impl(jobs)
}

fn irrelevant_context_ratio_for_jobs(jobs: &[&JobReport]) -> f64 {
	metrics::irrelevant_context_ratio_for_jobs_impl(jobs)
}

fn mean_score(jobs: &[JobReport]) -> f64 {
	metrics::mean_score_impl(jobs)
}

fn mean_latency(jobs: &[JobReport]) -> Option<f64> {
	metrics::mean_latency_impl(jobs)
}

fn consolidation_summary(jobs: &[JobReport]) -> ConsolidationSummaryReport {
	consolidation::consolidation_summary_impl(jobs)
}

fn memory_summary_summary(jobs: &[JobReport]) -> Option<MemorySummaryReport> {
	memory::memory_summary_summary_impl(jobs)
}

fn proactive_brief_summary(jobs: &[JobReport]) -> Option<ProactiveBriefSummaryReport> {
	proactive::proactive_brief_summary_impl(jobs)
}

fn scheduled_memory_summary(jobs: &[JobReport]) -> Option<ScheduledMemorySummaryReport> {
	scheduled::scheduled_memory_summary_impl(jobs)
}

fn work_continuity_summary(jobs: &[JobReport]) -> Option<WorkContinuitySummaryReport> {
	work::work_continuity_summary_impl(jobs)
}

fn knowledge_summary(jobs: &[JobReport]) -> Option<KnowledgeSummary> {
	knowledge::knowledge_summary_impl(jobs)
}
