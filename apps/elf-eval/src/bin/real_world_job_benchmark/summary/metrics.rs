use crate::{
	CostReport, JobReport, formatting,
	summary::{self},
};

pub(super) fn ratio_impl(numerator: usize, denominator: usize) -> f64 {
	if denominator == 0 {
		return 0.0;
	}

	formatting::round3(numerator as f64 / denominator as f64)
}

pub(super) fn expected_evidence_recall_for_jobs_impl(jobs: &[&JobReport]) -> f64 {
	let total = jobs.iter().map(|job| job.retrieval_quality.expected_evidence_total).sum::<usize>();
	let matched =
		jobs.iter().map(|job| job.retrieval_quality.expected_evidence_matched).sum::<usize>();

	summary::ratio_or(matched, total, 1.0)
}

pub(super) fn irrelevant_context_ratio_for_jobs_impl(jobs: &[&JobReport]) -> f64 {
	let total = jobs.iter().map(|job| job.retrieval_quality.produced_evidence_total).sum::<usize>();
	let irrelevant =
		jobs.iter().map(|job| job.retrieval_quality.irrelevant_context_count).sum::<usize>();

	summary::ratio_or(irrelevant, total, 0.0)
}

pub(super) fn ratio_or_impl(numerator: usize, denominator: usize, empty_value: f64) -> f64 {
	if denominator == 0 {
		empty_value
	} else {
		formatting::round3(numerator as f64 / denominator as f64)
	}
}

pub(super) fn ratio_or_full_impl(numerator: usize, denominator: usize) -> f64 {
	summary::ratio_or(numerator, denominator, 1.0)
}

pub(super) fn mean_score_impl(jobs: &[JobReport]) -> f64 {
	if jobs.is_empty() {
		return 0.0;
	}

	formatting::round3(jobs.iter().map(|job| job.normalized_score).sum::<f64>() / jobs.len() as f64)
}

pub(super) fn mean_latency_impl(jobs: &[JobReport]) -> Option<f64> {
	let latencies = jobs.iter().filter_map(|job| job.latency_ms).collect::<Vec<_>>();

	summary::mean_latency_for_values(latencies.as_slice())
}

pub(super) fn mean_latency_for_reports_impl(jobs: &[&JobReport]) -> Option<f64> {
	let latencies = jobs.iter().filter_map(|job| job.latency_ms).collect::<Vec<_>>();

	summary::mean_latency_for_values(latencies.as_slice())
}

pub(super) fn mean_latency_for_values_impl(latencies: &[f64]) -> Option<f64> {
	if latencies.is_empty() {
		None
	} else {
		Some(formatting::round3(latencies.iter().sum::<f64>() / latencies.len() as f64))
	}
}

pub(super) fn total_cost_impl(jobs: &[JobReport]) -> Option<CostReport> {
	let costs = jobs.iter().filter_map(|job| job.cost.as_ref()).collect::<Vec<_>>();

	total_cost_for_values(costs.as_slice())
}

pub(super) fn total_cost_for_reports_impl(jobs: &[&JobReport]) -> Option<CostReport> {
	let costs = jobs.iter().filter_map(|job| job.cost.as_ref()).collect::<Vec<_>>();

	total_cost_for_values(costs.as_slice())
}

pub(super) fn mean_proposal_metric_impl(values: impl Iterator<Item = f64>) -> Option<f64> {
	let values = values.collect::<Vec<_>>();

	if values.is_empty() {
		None
	} else {
		Some(formatting::round3(values.iter().sum::<f64>() / values.len() as f64))
	}
}

fn total_cost_for_values(costs: &[&CostReport]) -> Option<CostReport> {
	if costs.is_empty() {
		return None;
	}

	let currency = costs.iter().find_map(|cost| cost.currency.clone());
	let amount = sum_optional_f64(costs.iter().filter_map(|cost| cost.amount));
	let input_tokens = sum_optional_u64(costs.iter().filter_map(|cost| cost.input_tokens));
	let output_tokens = sum_optional_u64(costs.iter().filter_map(|cost| cost.output_tokens));

	Some(CostReport { currency, amount, input_tokens, output_tokens })
}

fn sum_optional_f64(values: impl Iterator<Item = f64>) -> Option<f64> {
	let values = values.collect::<Vec<_>>();

	if values.is_empty() { None } else { Some(formatting::round3(values.iter().sum())) }
}

fn sum_optional_u64(values: impl Iterator<Item = u64>) -> Option<u64> {
	let values = values.collect::<Vec<_>>();

	if values.is_empty() { None } else { Some(values.iter().sum()) }
}
