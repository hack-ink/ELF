use crate::quantitative::QUANTITATIVE_K_VALUES;

pub(super) fn quantitative_metric_names() -> Vec<String> {
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
