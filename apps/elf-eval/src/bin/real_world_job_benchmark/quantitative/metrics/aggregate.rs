mod confidence;
mod denominators;
mod metrics;
mod names;
mod states;

use crate::{BTreeMap, QuantitativeConfidenceInterval, QuantitativePerQueryRow};

pub(super) fn aggregate_metrics(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, Option<f64>> {
	metrics::aggregate_metrics(rows)
}

pub(super) fn aggregate_metric_states(
	result_state: &str,
	metric_comparable: bool,
) -> BTreeMap<String, String> {
	states::aggregate_metric_states(result_state, metric_comparable)
}

pub(super) fn aggregate_denominators(rows: &[QuantitativePerQueryRow]) -> BTreeMap<String, usize> {
	denominators::aggregate_denominators(rows)
}

pub(super) fn aggregate_confidence_intervals(
	rows: &[QuantitativePerQueryRow],
) -> BTreeMap<String, QuantitativeConfidenceInterval> {
	confidence::aggregate_confidence_intervals(rows)
}
