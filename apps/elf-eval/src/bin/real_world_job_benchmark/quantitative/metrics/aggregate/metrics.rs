use crate::{
	BTreeMap, QuantitativePerQueryRow, formatting, quantitative::metrics::aggregate::names,
};

pub(super) fn aggregate_metrics(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, Option<f64>> {
	let mut sums = BTreeMap::<String, (f64, usize)>::new();
	let mut metrics = names::quantitative_metric_names()
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
