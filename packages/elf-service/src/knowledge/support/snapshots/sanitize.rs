use crate::knowledge::support::{Map, Number, Value, serde_json};

pub(in crate::knowledge) fn sanitize_proposal_snapshot(source_snapshot: &Value) -> Value {
	let Some(object) = source_snapshot.as_object() else {
		return serde_json::json!({
			"kind": "proposal",
			"sanitized": true,
			"source_visibility": "proposal_metadata_only",
		});
	};
	let nested_source_count =
		object.get("source_refs").and_then(Value::as_array).map(Vec::len).unwrap_or_default();
	let mut sanitized = Map::new();

	for key in [
		"kind",
		"proposal_id",
		"run_id",
		"agent_id",
		"proposal_kind",
		"apply_intent",
		"review_state",
		"confidence",
		"proposed_payload_hash",
		"updated_at",
	] {
		if let Some(value) = object.get(key) {
			sanitized.insert(key.to_string(), value.clone());
		}
	}

	sanitized.insert("sanitized".to_string(), Value::Bool(true));
	sanitized.insert(
		"source_visibility".to_string(),
		Value::String("proposal_metadata_only".to_string()),
	);
	sanitized.insert(
		"omitted_fields".to_string(),
		serde_json::json!([
			"source_refs",
			"source_snapshot",
			"lineage",
			"diff",
			"unsupported_claim_flags",
			"contradiction_markers",
			"staleness_markers",
			"target_ref"
		]),
	);
	sanitized.insert(
		"nested_source_ref_count".to_string(),
		Value::Number(Number::from(nested_source_count)),
	);

	Value::Object(sanitized)
}
