use serde_json::Value;

use crate::app::server;

#[test]
fn docs_search_l0_schema_includes_filter_fields() {
	let schema = server::docs_search_l0_schema();
	let properties = schema
		.get("properties")
		.and_then(Value::as_object)
		.expect("docs_search_l0 schema is missing properties.");
	let required = ["query"];
	let expected = [
		"scope",
		"status",
		"doc_type",
		"agent_id",
		"thread_id",
		"updated_after",
		"updated_before",
		"ts_gte",
		"ts_lte",
		"sparse_mode",
		"domain",
		"repo",
		"explain",
	];

	for field in required {
		assert!(
			schema
				.get("required")
				.and_then(Value::as_array)
				.is_some_and(|fields| { fields.iter().any(|value| value.as_str() == Some(field)) }),
			"Missing required field {field}."
		);
	}
	for field in expected {
		assert!(properties.contains_key(field), "Missing schema field: {field}.");
	}

	assert_eq!(
		properties.get("status").and_then(Value::as_object).and_then(|status| {
			status.get("enum").and_then(Value::as_array).map(|vals| vals.to_vec())
		}),
		Some(vec![
			Value::String("active".to_string()),
			Value::String("deleted".to_string()),
			Value::Null,
		])
	);
	assert_eq!(
		properties.get("sparse_mode").and_then(Value::as_object).and_then(|field| {
			field.get("enum").and_then(Value::as_array).map(|vals| vals.to_vec())
		}),
		Some(vec![
			Value::String("auto".to_string()),
			Value::String("on".to_string()),
			Value::String("off".to_string()),
			Value::Null,
		])
	);
}
#[test]
fn docs_put_schema_includes_required_fields_and_write_policy() {
	let schema = server::docs_put_schema();
	let properties = schema
		.get("properties")
		.and_then(Value::as_object)
		.expect("docs_put schema is missing properties.");
	let required = ["scope", "content", "source_ref"];
	let expected = ["scope", "doc_type", "title", "source_ref", "write_policy", "content"];

	for field in required {
		assert!(
			schema
				.get("required")
				.and_then(Value::as_array)
				.is_some_and(|fields| { fields.iter().any(|value| value.as_str() == Some(field)) }),
			"Missing required field {field}."
		);
	}
	for field in expected {
		assert!(properties.contains_key(field), "Missing schema field: {field}.");
	}

	let write_policy = properties.get("write_policy").and_then(Value::as_object);
	let source_ref_properties = properties
		.get("source_ref")
		.and_then(|value| value.get("properties"))
		.and_then(Value::as_object)
		.expect("docs_put source_ref schema is missing properties.");

	assert!(
		write_policy.is_some_and(|field| {
			field.get("type").and_then(Value::as_array).is_some_and(|types| {
				types.contains(&Value::String("object".to_string()))
					&& types.contains(&Value::String("null".to_string()))
			})
		}),
		"Missing write_policy object/null type in docs_put schema."
	);

	for field in ["source_kind", "canonical_uri", "captured_at", "trust_label", "excerpt_locator"] {
		assert!(source_ref_properties.contains_key(field), "Missing source_ref field: {field}.");
	}
}
#[test]
fn docs_excerpts_get_schema_includes_l0_level_and_optional_explain() {
	let schema = server::docs_excerpts_get_schema();
	let properties = schema
		.get("properties")
		.and_then(Value::as_object)
		.expect("docs_excerpts_get schema is missing properties.");
	let level_values = properties
		.get("level")
		.and_then(|level| level.get("enum"))
		.and_then(|values| values.as_array())
		.expect("docs_excerpts_get level schema is missing enum.");

	assert!(level_values.contains(&Value::String("L0".to_string())));
	assert!(properties.contains_key("explain"));
}
