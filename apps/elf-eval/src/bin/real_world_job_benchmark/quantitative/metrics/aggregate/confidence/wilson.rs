use crate::{QuantitativeConfidenceInterval, formatting, quantitative::WILSON_95_Z};

pub(super) fn wilson_confidence_interval(
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
