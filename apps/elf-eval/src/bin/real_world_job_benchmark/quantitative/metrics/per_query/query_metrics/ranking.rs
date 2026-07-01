mod average_precision;
mod ndcg;
mod reciprocal_rank;

use crate::BTreeMap;

pub(super) fn reciprocal_rank(
	candidates: &[String],
	relevance: &BTreeMap<String, f64>,
) -> Option<f64> {
	reciprocal_rank::reciprocal_rank(candidates, relevance)
}

pub(super) fn ndcg_at_k(
	candidates: &[String],
	relevance: &BTreeMap<String, f64>,
	k: usize,
) -> Option<f64> {
	ndcg::ndcg_at_k(candidates, relevance, k)
}

pub(super) fn average_precision(
	candidates: &[String],
	relevance: &BTreeMap<String, f64>,
) -> Option<f64> {
	average_precision::average_precision(candidates, relevance)
}
