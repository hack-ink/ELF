mod aggregate;
mod per_query;
mod ranking;

use crate::{
	BTreeMap, BTreeSet, JobReport, QuantitativeConfidenceInterval, QuantitativePerQueryRow,
	RealWorldJob, ReportSummary,
};

pub(super) fn quantitative_per_query_rows(
	source_jobs: &[RealWorldJob],
	jobs: &[JobReport],
	corpus_id: &str,
	evidence_class: &str,
	adapter_id: &str,
) -> Vec<QuantitativePerQueryRow> {
	per_query::quantitative_per_query_rows(source_jobs, jobs, corpus_id, evidence_class, adapter_id)
}

pub(super) fn aggregate_metrics(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, Option<f64>> {
	aggregate::aggregate_metrics(rows)
}

pub(super) fn aggregate_metric_states(
	result_state: &str,
	metric_comparable: bool,
) -> BTreeMap<String, String> {
	aggregate::aggregate_metric_states(result_state, metric_comparable)
}

pub(super) fn aggregate_denominators(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, usize> {
	aggregate::aggregate_denominators(rows)
}

pub(super) fn aggregate_confidence_intervals(
	rows: &[QuantitativePerQueryRow],
) -> BTreeMap<String, QuantitativeConfidenceInterval> {
	aggregate::aggregate_confidence_intervals(rows)
}

pub(super) fn ranking_query_ids(source_jobs: &[RealWorldJob]) -> BTreeSet<&str> {
	ranking::ranking_query_ids(source_jobs)
}

pub(super) fn ranking_query_count(source_jobs: &[RealWorldJob]) -> usize {
	ranking::ranking_query_count(source_jobs)
}

pub(super) fn explicit_qrel_query_count(source_jobs: &[RealWorldJob]) -> usize {
	ranking::explicit_qrel_query_count(source_jobs)
}

pub(super) fn aggregate_qrel_source(
	ranking_query_count: usize,
	explicit_qrel_query_count: usize,
) -> &'static str {
	ranking::aggregate_qrel_source(ranking_query_count, explicit_qrel_query_count)
}

pub(super) fn ranking_coverage_state(
	summary: &ReportSummary,
	source_job_count: usize,
	ranking_query_count: usize,
) -> &'static str {
	ranking::ranking_coverage_state(summary, source_job_count, ranking_query_count)
}

pub(super) fn ranked_candidate_source(ranking_query_count: usize) -> &'static str {
	ranking::ranked_candidate_source(ranking_query_count)
}
