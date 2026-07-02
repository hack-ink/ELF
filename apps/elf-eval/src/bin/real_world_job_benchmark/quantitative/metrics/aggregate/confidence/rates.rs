use crate::{QuantitativePerQueryRow, quantitative::QUANTITATIVE_K_VALUES};

pub(super) fn rate_metric_names() -> Vec<String> {
	let mut metrics = Vec::new();

	for k in QUANTITATIVE_K_VALUES {
		metrics.push(format!("recall_at_{k}"));
		metrics.push(format!("precision_at_{k}"));
		metrics.push(format!("success_at_{k}"));
	}

	metrics
}

pub(super) fn aggregate_rate_numerator_denominator(
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
