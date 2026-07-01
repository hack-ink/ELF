use crate::{BTreeMap, BTreeSet, formatting, quantitative::QUANTITATIVE_K_VALUES};

pub(super) fn per_query_metrics(
	candidates: &[String],
	relevance: &BTreeMap<String, f64>,
) -> BTreeMap<String, Option<f64>> {
	let mut metrics = BTreeMap::new();

	for k in QUANTITATIVE_K_VALUES {
		let relevant_at_k = relevant_at_k(candidates, relevance, *k);

		metrics
			.insert(format!("recall_at_{k}"), rate(relevant_at_k, positive_qrel_count(relevance)));
		metrics.insert(format!("precision_at_{k}"), rate(relevant_at_k, *k));
		metrics.insert(
			format!("success_at_{k}"),
			Some(f64::from(relevant_at_k > 0 && positive_qrel_count(relevance) > 0)),
		);
	}

	metrics.insert("mrr".to_string(), reciprocal_rank(candidates, relevance));
	metrics.insert("ndcg_at_5".to_string(), ndcg_at_k(candidates, relevance, 5));
	metrics.insert("average_precision".to_string(), average_precision(candidates, relevance));

	metrics
}

pub(super) fn positive_qrel_count(relevance: &BTreeMap<String, f64>) -> usize {
	relevance.values().filter(|grade| **grade > 0.0).count()
}

pub(super) fn per_query_denominators(
	candidate_count: usize,
	expected_relevant_count: usize,
) -> BTreeMap<String, usize> {
	let mut denominators = BTreeMap::new();

	for k in QUANTITATIVE_K_VALUES {
		denominators.insert(format!("recall_at_{k}"), expected_relevant_count);
		denominators.insert(format!("precision_at_{k}"), *k);
		denominators.insert(format!("success_at_{k}"), 1);
	}

	denominators.insert("mrr".to_string(), expected_relevant_count);
	denominators.insert("ndcg_at_5".to_string(), expected_relevant_count.min(5));
	denominators.insert("average_precision".to_string(), expected_relevant_count);
	denominators.insert("candidate_count".to_string(), candidate_count);

	denominators
}

fn relevant_at_k(candidates: &[String], relevance: &BTreeMap<String, f64>, k: usize) -> usize {
	candidates
		.iter()
		.take(k)
		.filter(|candidate| relevance.get(candidate.as_str()).is_some_and(|grade| *grade > 0.0))
		.count()
}

fn reciprocal_rank(candidates: &[String], relevance: &BTreeMap<String, f64>) -> Option<f64> {
	if positive_qrel_count(relevance) == 0 {
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

fn ndcg_at_k(candidates: &[String], relevance: &BTreeMap<String, f64>, k: usize) -> Option<f64> {
	if positive_qrel_count(relevance) == 0 {
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

fn average_precision(candidates: &[String], relevance: &BTreeMap<String, f64>) -> Option<f64> {
	let positive_count = positive_qrel_count(relevance);

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

fn rate(numerator: usize, denominator: usize) -> Option<f64> {
	(denominator > 0).then(|| formatting::round3(numerator as f64 / denominator as f64))
}
