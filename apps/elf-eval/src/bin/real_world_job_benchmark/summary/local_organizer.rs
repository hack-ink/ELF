use std::collections::BTreeMap;

use crate::{
	CostReport, JobReport, LocalOrganizerJobReport, LocalOrganizerSummaryReport,
	LocalOrganizerTierReport, Value, formatting,
	summary::{self},
};

pub(super) fn local_organizer_summary_impl(
	jobs: &[JobReport],
) -> Option<LocalOrganizerSummaryReport> {
	let reports = jobs.iter().filter_map(|job| job.local_organizer.as_ref()).collect::<Vec<_>>();

	if reports.is_empty() {
		return None;
	}

	let source_mutation_count = reports.iter().map(|report| report.source_mutation_count).sum();
	let silent_memory_authority_mutation_count =
		reports.iter().map(|report| report.silent_memory_authority_mutation_count).sum();
	let citation_source_ref_coverage = mean_metric(
		reports.iter().map(|report| report.citation_source_ref_coverage).collect::<Vec<_>>(),
	);
	let unsupported_claim_rate =
		mean_metric(reports.iter().map(|report| report.unsupported_claim_rate).collect::<Vec<_>>());
	let stale_correction_delete_score = mean_metric(
		reports.iter().map(|report| report.stale_correction_delete_score).collect::<Vec<_>>(),
	);
	let escalation_rate =
		mean_metric(reports.iter().map(|report| report.escalation_rate).collect::<Vec<_>>());
	let latencies = reports.iter().filter_map(|report| report.latency_ms).collect::<Vec<_>>();
	let extraction_f1_not_encoded_count =
		reports.iter().filter(|report| report.extraction_f1.is_none()).count();
	let json_schema_valid_count = reports.iter().filter(|report| report.json_schema_valid).count();

	Some(LocalOrganizerSummaryReport {
		job_count: reports.len(),
		extraction_f1_not_encoded_count,
		json_schema_valid_count,
		citation_source_ref_coverage,
		unsupported_claim_rate,
		stale_correction_delete_score,
		source_mutation_count,
		silent_memory_authority_mutation_count,
		escalation_rate,
		mean_latency_ms: summary::mean_latency_for_values(latencies.as_slice()),
		p95_latency_ms: percentile_latency(latencies, 0.95),
		tiers: tier_reports(&reports),
	})
}

fn tier_reports(reports: &[&LocalOrganizerJobReport]) -> Vec<LocalOrganizerTierReport> {
	let mut by_tier = BTreeMap::<String, Vec<&LocalOrganizerJobReport>>::new();

	for report in reports {
		by_tier.entry(report.model_tier.clone()).or_default().push(*report);
	}

	by_tier
		.into_iter()
		.map(|(model_tier, reports)| {
			let latencies =
				reports.iter().filter_map(|report| report.latency_ms).collect::<Vec<_>>();
			let costs =
				reports.iter().filter_map(|report| report.cost.as_ref()).collect::<Vec<_>>();
			let resource_footprints = reports
				.iter()
				.map(|report| report.resource_footprint.clone())
				.filter(|footprint| !footprint.is_null())
				.collect::<Vec<Value>>();

			LocalOrganizerTierReport {
				model_tier,
				job_count: reports.len(),
				mean_latency_ms: summary::mean_latency_for_values(latencies.as_slice()),
				p95_latency_ms: percentile_latency(latencies, 0.95),
				total_cost: total_cost_for_values(costs.as_slice()),
				resource_footprints,
			}
		})
		.collect()
}

fn mean_metric(values: Vec<f64>) -> f64 {
	if values.is_empty() {
		0.0
	} else {
		formatting::round3(values.iter().sum::<f64>() / values.len() as f64)
	}
}

fn percentile_latency(mut values: Vec<f64>, percentile: f64) -> Option<f64> {
	if values.is_empty() {
		return None;
	}

	values.sort_by(|left, right| left.total_cmp(right));

	let index = ((values.len() as f64 - 1.0) * percentile).ceil() as usize;

	values.get(index).copied().map(formatting::round3)
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
