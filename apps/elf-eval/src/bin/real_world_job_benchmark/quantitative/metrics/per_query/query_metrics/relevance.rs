use crate::{BTreeMap, formatting};

pub(in crate::quantitative::metrics::per_query) fn positive_qrel_count(
	relevance: &BTreeMap<String, f64>,
) -> usize {
	relevance.values().filter(|grade| **grade > 0.0).count()
}

pub(super) fn relevant_at_k(
	candidates: &[String],
	relevance: &BTreeMap<String, f64>,
	k: usize,
) -> usize {
	candidates
		.iter()
		.take(k)
		.filter(|candidate| relevance.get(candidate.as_str()).is_some_and(|grade| *grade > 0.0))
		.count()
}

pub(super) fn rate(numerator: usize, denominator: usize) -> Option<f64> {
	(denominator > 0).then(|| formatting::round3(numerator as f64 / denominator as f64))
}
