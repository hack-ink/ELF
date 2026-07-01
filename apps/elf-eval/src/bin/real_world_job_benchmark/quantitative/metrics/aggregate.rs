mod confidence;

use crate::{
	BTreeMap, QuantitativeConfidenceInterval, QuantitativePerQueryRow, formatting,
	quantitative::QUANTITATIVE_K_VALUES,
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
	confidence::aggregate_confidence_intervals(rows)
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

fn sum_per_query_denominator(rows: &[QuantitativePerQueryRow], metric: &str) -> usize {
	rows.iter().filter_map(|row| row.denominators.get(metric)).sum()
}
