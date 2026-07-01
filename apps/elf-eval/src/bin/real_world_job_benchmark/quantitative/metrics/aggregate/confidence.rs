use crate::{
	BTreeMap, QuantitativeConfidenceInterval, QuantitativePerQueryRow, formatting,
	quantitative::{QUANTITATIVE_K_VALUES, WILSON_95_Z},
};

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
