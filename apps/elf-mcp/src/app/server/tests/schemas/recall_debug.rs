use serde_json::Value;

use crate::app::server;

#[test]
fn recall_debug_panel_schema_rejects_context_override_fields() {
	let schema = server::recall_debug_panel_schema();
	let properties = schema
		.get("properties")
		.and_then(Value::as_object)
		.expect("recall debug panel schema is missing properties.");

	assert_eq!(schema.get("additionalProperties"), Some(&Value::Bool(false)));

	for key in ["tenant_id", "project_id", "agent_id", "read_profile"] {
		assert!(!properties.contains_key(key), "{key} must not be a tool param.");
	}
	for key in ["graph_subject", "graph_predicate"] {
		let one_of = properties
			.get(key)
			.and_then(Value::as_object)
			.and_then(|schema| schema.get("oneOf"))
			.and_then(Value::as_array)
			.expect("selector schema is missing oneOf.");

		for branch in one_of.iter().filter_map(Value::as_object) {
			if branch.get("type").and_then(Value::as_str) == Some("object") {
				assert_eq!(
					branch.get("additionalProperties"),
					Some(&Value::Bool(false)),
					"{key} selector object branches must be closed."
				);
			}
		}
	}
}
