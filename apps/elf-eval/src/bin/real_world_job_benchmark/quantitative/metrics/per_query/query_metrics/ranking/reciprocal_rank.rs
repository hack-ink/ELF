use crate::{BTreeMap, quantitative::metrics::per_query::query_metrics};

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
