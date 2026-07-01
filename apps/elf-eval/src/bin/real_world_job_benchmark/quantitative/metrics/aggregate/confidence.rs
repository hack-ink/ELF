mod rates;
mod wilson;

use crate::{BTreeMap, QuantitativeConfidenceInterval, QuantitativePerQueryRow};

pub(super) fn aggregate_confidence_intervals(
	rows: &[QuantitativePerQueryRow],
) -> BTreeMap<String, QuantitativeConfidenceInterval> {
	let mut confidence_intervals = BTreeMap::new();

	for metric in rates::rate_metric_names() {
		let (numerator, denominator) =
			rates::aggregate_rate_numerator_denominator(rows, metric.as_str());

		if denominator > 0 {
			confidence_intervals.insert(
				metric,
				wilson::wilson_confidence_interval(numerator.min(denominator), denominator),
			);
		}
	}

	confidence_intervals
}
