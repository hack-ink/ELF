use crate::{BTreeMap, QuantitativePerQueryRow, quantitative::QUANTITATIVE_K_VALUES};

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

fn sum_per_query_denominator(rows: &[QuantitativePerQueryRow], metric: &str) -> usize {
	rows.iter().filter_map(|row| row.denominators.get(metric)).sum()
}
