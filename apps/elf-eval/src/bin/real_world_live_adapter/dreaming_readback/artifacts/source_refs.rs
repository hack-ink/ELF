use crate::Value;

pub(in crate::dreaming_readback) fn collect_dreaming_artifact_source_refs(
	value: &Value,
	refs: &mut Vec<String>,
) {
	match value {
		Value::Array(items) =>
			for item in items {
				collect_dreaming_artifact_source_refs(item, refs);
			},
		Value::Object(map) =>
			for (key, value) in map {
				collect_named_source_refs(key, value, refs);
				collect_dreaming_artifact_source_refs(value, refs);
			},
		_ => {},
	}
}

fn collect_named_source_refs(key: &str, value: &Value, refs: &mut Vec<String>) {
	if matches!(key, "source_refs" | "evidence_refs" | "evidence_ids")
		&& let Some(items) = value.as_array()
	{
		for item in items {
			if let Some(source_ref) = item.as_str() {
				crate::push_unique(refs, source_ref.to_string());
			}
		}
	}
	if key == "evidence_id"
		&& let Some(source_ref) = value.as_str()
	{
		crate::push_unique(refs, source_ref.to_string());
	}
}
