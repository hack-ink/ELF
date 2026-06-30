use serde_json::Value;

use crate::structured_fields::StructuredFields;
use elf_domain::english_gate;

pub(super) fn find_non_english_path_in_structured(
	structured: Option<&StructuredFields>,
	base: &str,
) -> Option<String> {
	let structured = structured?;

	if let Some(summary) = structured.summary.as_ref()
		&& !english_gate::is_english_natural_language(summary)
	{
		return Some(format!("{base}.summary"));
	}
	if let Some(items) = structured.facts.as_ref() {
		for (idx, item) in items.iter().enumerate() {
			if !english_gate::is_english_natural_language(item) {
				return Some(format!("{base}.facts[{idx}]"));
			}
		}
	}
	if let Some(items) = structured.concepts.as_ref() {
		for (idx, item) in items.iter().enumerate() {
			if !english_gate::is_english_natural_language(item) {
				return Some(format!("{base}.concepts[{idx}]"));
			}
		}
	}
	if let Some(items) = structured.entities.as_ref() {
		for (idx, entity) in items.iter().enumerate() {
			let base = format!("{base}.entities[{idx}]");

			if let Some(canonical) = entity.canonical.as_ref()
				&& !english_gate::is_english_natural_language(canonical)
			{
				return Some(format!("{base}.canonical"));
			}
			if let Some(kind) = entity.kind.as_ref()
				&& !english_gate::is_english_natural_language(kind)
			{
				return Some(format!("{base}.kind"));
			}
			if let Some(aliases) = entity.aliases.as_ref() {
				for (alias_idx, alias) in aliases.iter().enumerate() {
					if !english_gate::is_english_natural_language(alias) {
						return Some(format!("{base}.aliases[{alias_idx}]"));
					}
				}
			}
		}
	}
	if let Some(items) = structured.relations.as_ref() {
		for (idx, relation) in items.iter().enumerate() {
			let base = format!("{base}.relations[{idx}]");

			if let Some(subject) = relation.subject.as_ref() {
				let subject_base = format!("{base}.subject");

				if let Some(canonical) = subject.canonical.as_ref()
					&& !english_gate::is_english_natural_language(canonical)
				{
					return Some(format!("{subject_base}.canonical"));
				}
				if let Some(kind) = subject.kind.as_ref()
					&& !english_gate::is_english_natural_language(kind)
				{
					return Some(format!("{subject_base}.kind"));
				}
				if let Some(aliases) = subject.aliases.as_ref() {
					for (alias_idx, alias) in aliases.iter().enumerate() {
						if !english_gate::is_english_natural_language(alias) {
							return Some(format!("{subject_base}.aliases[{alias_idx}]"));
						}
					}
				}
			}
			if let Some(predicate) = relation.predicate.as_ref()
				&& !english_gate::is_english_natural_language(predicate)
			{
				return Some(format!("{base}.predicate"));
			}
			if let Some(object) = relation.object.as_ref() {
				if let Some(entity) = object.entity.as_ref() {
					let object_base = format!("{base}.object.entity");

					if let Some(canonical) = entity.canonical.as_ref()
						&& !english_gate::is_english_natural_language(canonical)
					{
						return Some(format!("{object_base}.canonical"));
					}
					if let Some(kind) = entity.kind.as_ref()
						&& !english_gate::is_english_natural_language(kind)
					{
						return Some(format!("{object_base}.kind"));
					}
					if let Some(aliases) = entity.aliases.as_ref() {
						for (alias_idx, alias) in aliases.iter().enumerate() {
							if !english_gate::is_english_natural_language(alias) {
								return Some(format!("{object_base}.aliases[{alias_idx}]"));
							}
						}
					}
				}
				if let Some(value) = object.value.as_ref()
					&& !english_gate::is_english_natural_language(value)
				{
					return Some(format!("{base}.object.value"));
				}
			}
		}
	}

	None
}

pub(super) fn find_non_english_path(value: &Value, path: &str) -> Option<String> {
	find_non_english_path_inner(value, path, true)
}

fn find_non_english_path_inner(
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

fn escape_json_path_key(key: &str) -> String {
	key.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)] mod tests;
