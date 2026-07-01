use crate::{BTreeMap, BTreeSet, quantitative::metrics::per_query::query_metrics};

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
