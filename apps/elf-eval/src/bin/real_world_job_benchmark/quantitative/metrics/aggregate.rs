use crate::{
	BTreeMap, QuantitativeConfidenceInterval, QuantitativePerQueryRow, formatting,
	quantitative::{QUANTITATIVE_K_VALUES, WILSON_95_Z},
};

pub(super) fn aggregate_metrics(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, Option<f64>> {
	let mut sums = BTreeMap::<String, (f64, usize)>::new();
	let mut metrics = quantitative_metric_names()
		.into_iter()
		.map(|metric| (metric, None))
		.collect::<BTreeMap<_, _>>();

	for row in rows {
		for (metric, value) in &row.metrics {
			if let Some(value) = value {
				let (sum, count) = sums.entry(metric.clone()).or_default();

				*sum += *value;
				*count += 1;
			}
		}
	}
	for (metric, (sum, count)) in sums {
		metrics.insert(metric, (count > 0).then(|| formatting::round3(sum / count as f64)));
	}

	metrics
}

pub(super) fn aggregate_metric_states(
	result_state: &str,
	metric_comparable: bool,
) -> BTreeMap<String, String> {
	let state = if metric_comparable { result_state } else { "not_encoded" };
	let mut states = BTreeMap::new();

	for k in QUANTITATIVE_K_VALUES {
		states.insert(format!("recall_at_{k}"), state.to_string());
		states.insert(format!("precision_at_{k}"), state.to_string());
		states.insert(format!("success_at_{k}"), state.to_string());
	}
	for metric in ["mrr", "ndcg_at_5", "average_precision"] {
		states.insert(metric.to_string(), state.to_string());
	}

	states
}

pub(super) fn aggregate_denominators(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, usize> {
	let mut denominators = BTreeMap::new();

	for k in QUANTITATIVE_K_VALUES {
		denominators.insert(
			format!("recall_at_{k}"),
			sum_per_query_denominator(rows, &format!("recall_at_{k}")),
		);
		denominators.insert(
			format!("precision_at_{k}"),
			sum_per_query_denominator(rows, &format!("precision_at_{k}")),
		);
		denominators.insert(
			format!("success_at_{k}"),
			sum_per_query_denominator(rows, &format!("success_at_{k}")),
		);
	}

	denominators.insert("mrr".to_string(), sum_per_query_denominator(rows, "mrr"));
	denominators.insert("ndcg_at_5".to_string(), sum_per_query_denominator(rows, "ndcg_at_5"));
	denominators.insert(
		"average_precision".to_string(),
		sum_per_query_denominator(rows, "average_precision"),
	);

	denominators
}

pub(super) fn aggregate_confidence_intervals(
	rows: &[QuantitativePerQueryRow],
) -> BTreeMap<String, QuantitativeConfidenceInterval> {
	let mut confidence_intervals = BTreeMap::new();

	for metric in rate_metric_names() {
		let (numerator, denominator) = aggregate_rate_numerator_denominator(rows, metric.as_str());

		if denominator > 0 {
			confidence_intervals.insert(
				metric,
				wilson_confidence_interval(numerator.min(denominator), denominator),
			);
		}
	}

	confidence_intervals
}

fn quantitative_metric_names() -> Vec<String> {
	let mut metrics = Vec::new();

	for k in QUANTITATIVE_K_VALUES {
		metrics.push(format!("recall_at_{k}"));
		metrics.push(format!("precision_at_{k}"));
		metrics.push(format!("success_at_{k}"));
	}
	for metric in ["mrr", "ndcg_at_5", "average_precision"] {
		metrics.push(metric.to_string());
	}

	metrics
}

fn rate_metric_names() -> Vec<String> {
	let mut metrics = Vec::new();

	for k in QUANTITATIVE_K_VALUES {
		metrics.push(format!("recall_at_{k}"));
		metrics.push(format!("precision_at_{k}"));
		metrics.push(format!("success_at_{k}"));
	}

	metrics
}

fn aggregate_rate_numerator_denominator(
	rows: &[QuantitativePerQueryRow],
	metric: &str,
) -> (usize, usize) {
	let mut numerator = 0;
	let mut denominator = 0;

	for row in rows {
		let Some(value) = row.metrics.get(metric).and_then(|value| *value) else {
			continue;
		};
		let Some(row_denominator) = row.denominators.get(metric).copied() else {
			continue;
		};

		if row_denominator == 0 {
			continue;
		}

		denominator += row_denominator;
		numerator += (value * row_denominator as f64).round() as usize;
	}

	(numerator, denominator)
}

fn wilson_confidence_interval(
	numerator: usize,
	denominator: usize,
) -> QuantitativeConfidenceInterval {
	let n = denominator as f64;
	let p = numerator as f64 / n;
	let z2 = WILSON_95_Z * WILSON_95_Z;
	let center = (p + z2 / (2.0 * n)) / (1.0 + z2 / n);
	let half_width =
		WILSON_95_Z * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / (1.0 + z2 / n);

	QuantitativeConfidenceInterval {
		method: "wilson_score".to_string(),
		confidence: 0.95,
		lower: formatting::round3((center - half_width).clamp(0.0, 1.0)),
		upper: formatting::round3((center + half_width).clamp(0.0, 1.0)),
		numerator,
		denominator,
	}
}

fn sum_per_query_denominator(rows: &[QuantitativePerQueryRow], metric: &str) -> usize {
	rows.iter().filter_map(|row| row.denominators.get(metric)).sum()
}
