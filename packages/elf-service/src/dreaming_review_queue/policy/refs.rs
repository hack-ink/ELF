use serde_json::Value;

const FORBIDDEN_SOURCE_MUTATION_KEYS: [&str; 8] = [
	"delete_source",
	"delete_sources",
	"overwrite_source",
	"source_delete",
	"source_mutation",
	"source_mutations",
	"source_note_updates",
	"update_source",
];

pub(in crate::dreaming_review_queue) fn affected_refs(
	target_ref: &Value,
	proposed_payload: &Value,
) -> Vec<Value> {
	let mut refs = Vec::new();

	push_non_empty_object(&mut refs, target_ref);

	for pointer in [
		"/affected_refs",
		"/affected_pages",
		"/affected_memories",
		"/affected_facts",
		"/affected_notes",
	] {
		match proposed_payload.pointer(pointer) {
			Some(Value::Array(values)) => refs.extend(values.iter().cloned()),
			Some(value) if non_empty_json_object(value) => refs.push(value.clone()),
			_ => {},
		}
	}

	refs
}

pub(in crate::dreaming_review_queue) fn non_empty_json_array(value: &Value) -> bool {
	value.as_array().is_some_and(|array| !array.is_empty())
}

pub(in crate::dreaming_review_queue) fn contains_forbidden_source_mutation_key(
	value: &Value,
) -> bool {
	match value {
		Value::Object(map) => map.iter().any(|(key, nested)| {
			FORBIDDEN_SOURCE_MUTATION_KEYS.contains(&key.as_str())
				|| contains_forbidden_source_mutation_key(nested)
		}),
		Value::Array(items) => items.iter().any(contains_forbidden_source_mutation_key),
		_ => false,
	}
}

fn push_non_empty_object(refs: &mut Vec<Value>, value: &Value) {
	if non_empty_json_object(value) {
		refs.push(value.clone());
	}
}

fn non_empty_json_object(value: &Value) -> bool {
	value.as_object().is_some_and(|object| !object.is_empty())
}
