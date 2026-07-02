use crate::BTreeMap;

pub(super) fn per_query_metric_states<'a>(
	metric_names: impl Iterator<Item = &'a String>,
	positive_relevance_count: usize,
	candidate_count: usize,
	result_state: &str,
) -> BTreeMap<String, String> {
	let metric_state = if positive_relevance_count == 0 || candidate_count == 0 {
		"not_encoded"
	} else {
		result_state
	};

	metric_names.map(|key| (key.clone(), metric_state.to_string())).collect()
}
