use crate::docs::validation::{Value, english_gate};

pub(in crate::docs) fn find_non_english_path(value: &Value, path: &str) -> Option<String> {
	find_non_english_path_inner(value, path, false)
}

pub(in crate::docs) fn find_non_english_path_inner(
	value: &Value,
	path: &str,
	is_identifier_lane: bool,
) -> Option<String> {
	fn has_english_gate(text: &str, is_identifier_lane: bool) -> bool {
		if is_identifier_lane {
			return english_gate::is_english_identifier(text);
		}

		english_gate::is_english_natural_language(text)
	}

	match value {
		Value::String(text) =>
			if !has_english_gate(text, is_identifier_lane) {
				Some(path.to_string())
			} else {
				None
			},
		Value::Array(items) => {
			for (idx, item) in items.iter().enumerate() {
				let child_path = format!("{path}[{idx}]");

				if let Some(found) =
					find_non_english_path_inner(item, &child_path, is_identifier_lane)
				{
					return Some(found);
				}
			}

			None
		},
		Value::Object(map) => {
			for (key, value) in map.iter() {
				let identifier_lane = is_identifier_lane
					|| matches!(key.as_str(), "ref" | "schema" | "resolver" | "hashes" | "state");
				let child_path = format!("{path}[\"{}\"]", escape_json_path_key(key));

				if let Some(found) =
					find_non_english_path_inner(value, &child_path, identifier_lane)
				{
					return Some(found);
				}
			}

			None
		},
		_ => None,
	}
}

pub(in crate::docs) fn escape_json_path_key(key: &str) -> String {
	key.replace('\\', "\\\\").replace('"', "\\\"")
}
