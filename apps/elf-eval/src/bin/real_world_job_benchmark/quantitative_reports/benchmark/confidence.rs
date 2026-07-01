use crate::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct QuantitativeConfidenceInterval {
	pub(crate) method: String,
	pub(crate) confidence: f64,
	pub(crate) lower: f64,
	pub(crate) upper: f64,
	pub(crate) numerator: usize,
	pub(crate) denominator: usize,
}
