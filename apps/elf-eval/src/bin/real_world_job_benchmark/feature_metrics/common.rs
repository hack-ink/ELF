use crate::feature_metrics::{FORBIDDEN_SOURCE_MUTATION_KEYS, Value};

pub(super) fn forbidden_diff_key_count_impl(value: &Value) -> usize {
	match value {
		Value::Object(map) => map
			.iter()
			.map(|(key, nested)| {
				usize::from(FORBIDDEN_SOURCE_MUTATION_KEYS.contains(&key.as_str()))
					+ forbidden_diff_key_count_impl(nested)
			})
			.sum(),
		Value::Array(items) => items.iter().map(forbidden_diff_key_count_impl).sum(),
		_ => 0,
	}
}
