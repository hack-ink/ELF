mod denominators;
mod ranking;
mod relevance;

pub(super) use self::{denominators::per_query_denominators, relevance::positive_qrel_count};

use crate::{BTreeMap, quantitative::QUANTITATIVE_K_VALUES};

pub(super) fn per_query_metrics(
	candidates: &[String],
	relevance: &BTreeMap<String, f64>,
) -> BTreeMap<String, Option<f64>> {
	let mut metrics = BTreeMap::new();

	for k in QUANTITATIVE_K_VALUES {
		let relevant_at_k = relevance::relevant_at_k(candidates, relevance, *k);

		metrics.insert(
			format!("recall_at_{k}"),
			relevance::rate(relevant_at_k, positive_qrel_count(relevance)),
		);
		metrics.insert(format!("precision_at_{k}"), relevance::rate(relevant_at_k, *k));
		metrics.insert(
			format!("success_at_{k}"),
			Some(f64::from(relevant_at_k > 0 && positive_qrel_count(relevance) > 0)),
		);
	}

	metrics.insert("mrr".to_string(), ranking::reciprocal_rank(candidates, relevance));
	metrics.insert("ndcg_at_5".to_string(), ranking::ndcg_at_k(candidates, relevance, 5));
	metrics
		.insert("average_precision".to_string(), ranking::average_precision(candidates, relevance));

	metrics
}
