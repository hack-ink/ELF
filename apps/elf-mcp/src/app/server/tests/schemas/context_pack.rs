use serde_json::Value;

use crate::app::server;

#[test]
fn context_pack_schema_rejects_context_override_fields() {
	let schema = server::context_pack_build_schema();
	let properties = schema
		.get("properties")
		.and_then(Value::as_object)
		.expect("context pack schema is missing properties.");
	let required = schema
		.get("required")
		.and_then(Value::as_array)
		.expect("context pack schema is missing required fields.");

	assert_eq!(schema.get("additionalProperties"), Some(&Value::Bool(false)));
	assert!(required.iter().any(|value| value.as_str() == Some("task")));

	for key in ["tenant_id", "project_id", "agent_id", "read_profile"] {
		assert!(!properties.contains_key(key), "{key} must not be a tool param.");
	}

	assert!(
		!properties.contains_key("debug_overrides"),
		"debug overrides must not be public MCP params."
	);

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
