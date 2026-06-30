use crate::{
	JobReport, OperationalCostSummary, OperationalLatencyReport, formatting::round3, summary,
};

pub(in crate::operational) fn operational_latency_report(
	reports: &[JobReport],
) -> OperationalLatencyReport {
	let latencies = reports.iter().filter_map(|report| report.latency_ms).collect::<Vec<_>>();

	OperationalLatencyReport {
		measured_job_count: latencies.len(),
		missing_latency_job_count: reports.len().saturating_sub(latencies.len()),
		mean_ms: summary::mean_latency_for_values(latencies.as_slice()),
		max_ms: latencies.iter().copied().reduce(f64::max).map(round3),
	}
}

pub(in crate::operational) fn operational_cost_summary(
	reports: &[JobReport],
) -> OperationalCostSummary {
	let costs = reports.iter().filter_map(|report| report.cost.as_ref()).collect::<Vec<_>>();
	let zero_cost_job_count =
		costs.iter().filter(|cost| cost.amount.is_some_and(|amount| amount == 0.0)).count();

	OperationalCostSummary {
		jobs_with_cost_report: costs.len(),
		missing_cost_job_count: reports.len().saturating_sub(costs.len()),
		zero_cost_job_count,
		total: summary::total_cost(reports),
		claim_boundary: "Fixture and local-provider zero-cost reports are execution-accounting evidence only; they do not prove hosted provider spend.".to_string(),
	}
}
