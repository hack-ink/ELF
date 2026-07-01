use crate::{BTreeMap, BTreeSet, quantitative::metrics::per_query::query_metrics};

pub(super) fn reciprocal_rank(
	candidates: &[String],
	relevance: &BTreeMap<String, f64>,
) -> Option<f64> {
	if query_metrics::positive_qrel_count(relevance) == 0 {
		return None;
	}

	Some(
		candidates
			.iter()
			.position(|candidate| {
				relevance.get(candidate.as_str()).is_some_and(|grade| *grade > 0.0)
			})
			.map_or(0.0, |index| 1.0 / (index + 1) as f64),
	)
}

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

pub(super) fn average_precision(
	candidates: &[String],
	relevance: &BTreeMap<String, f64>,
) -> Option<f64> {
	let positive_count = query_metrics::positive_qrel_count(relevance);

	if positive_count == 0 {
		return None;
	}

	let mut hit_count = 0;
	let mut precision_sum = 0.0;
	let mut seen = BTreeSet::new();

	for (index, candidate) in candidates.iter().enumerate() {
		if !seen.insert(candidate.as_str()) {
			continue;
		}
		if relevance.get(candidate.as_str()).is_some_and(|grade| *grade > 0.0) {
			hit_count += 1;
			precision_sum += hit_count as f64 / (index + 1) as f64;
		}
	}

	Some(precision_sum / positive_count as f64)
}
