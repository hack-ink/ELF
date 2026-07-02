use crate::{BTreeMap, quantitative::metrics::per_query::query_metrics};

pub(super) fn ndcg_at_k(
	candidates: &[String],
	relevance: &BTreeMap<String, f64>,
	k: usize,
) -> Option<f64> {
	if query_metrics::positive_qrel_count(relevance) == 0 {
		return None;
	}

	let dcg = candidates
		.iter()
		.take(k)
		.enumerate()
		.map(|(index, candidate)| {
			relevance.get(candidate.as_str()).copied().unwrap_or(0.0).max(0.0)
				/ ((index + 2) as f64).log2()
		})
		.sum::<f64>();
	let mut ideal = relevance.values().copied().filter(|grade| *grade > 0.0).collect::<Vec<_>>();

	ideal.sort_by(|left, right| right.total_cmp(left));

	let idcg = ideal
		.iter()
		.take(k)
		.enumerate()
		.map(|(index, grade)| grade / ((index + 2) as f64).log2())
		.sum::<f64>();

	Some(if idcg > 0.0 { dcg / idcg } else { 0.0 })
}
