use serde_json::Value;

use crate::app::server;

#[test]
fn notes_ingest_schema_includes_structured_entities_relations() {
	let schema = server::notes_ingest_schema();
	let notes = schema
		.get("properties")
		.and_then(Value::as_object)
		.expect("notes ingest schema is missing properties.")
		.get("notes")
		.and_then(Value::as_object)
		.expect("notes schema is missing notes.");
	let note_items =
		notes.get("items").and_then(Value::as_object).expect("notes schema is missing items.");
	let note_properties = note_items
		.get("properties")
		.and_then(Value::as_object)
		.expect("notes schema is missing note item properties.");
	let structured = note_properties
		.get("structured")
		.and_then(Value::as_object)
		.expect("notes schema is missing structured.");
	let structured_type =
		structured.get("type").and_then(Value::as_array).expect("structured.type is not an array.");

	assert!(
		structured_type.contains(&Value::String("object".to_string()))
			&& structured_type.contains(&Value::String("null".to_string()))
	);

	let structured_properties = structured
		.get("properties")
		.and_then(Value::as_object)
		.expect("structured schema is missing properties.");

	assert!(structured_properties.contains_key("entities"));
	assert!(structured_properties.contains_key("relations"));

	let relation_object = structured_properties
		.get("relations")
		.and_then(Value::as_object)
		.and_then(|relations| relations.get("items"))
		.and_then(Value::as_object)
		.and_then(|items| items.get("properties"))
		.and_then(Value::as_object)
		.expect("relations schema is missing properties.")
		.get("object")
		.and_then(Value::as_object)
		.expect("relation schema is missing object.");
	let one_of = relation_object
		.get("oneOf")
		.and_then(Value::as_array)
		.expect("relation object is missing oneOf.");

	assert_eq!(one_of.len(), 2, "relation object should have entity/value oneOf variants.");
	assert!(one_of.iter().any(|variant| {
		variant.as_object().is_some_and(|branch| {
			branch
				.get("required")
				.and_then(Value::as_array)
				.is_some_and(|required| required.iter().any(|value| value == "entity"))
		})
	}));
	assert!(one_of.iter().any(|variant| {
		variant.as_object().is_some_and(|branch| {
			branch
				.get("required")
				.and_then(Value::as_array)
				.is_some_and(|required| required.iter().any(|value| value == "value"))
		})
	}));
}

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
fn work_journal_schemas_include_families_and_source_refs() {
	let create_schema = server::work_journal_entry_create_schema();
	let create_properties = create_schema
		.get("properties")
		.and_then(Value::as_object)
		.expect("work_journal_entry_create schema is missing properties.");
	let readback_schema = server::work_journal_session_readback_schema();
	let readback_properties = readback_schema
		.get("properties")
		.and_then(Value::as_object)
		.expect("work_journal_session_readback schema is missing properties.");

	for field in ["scope", "session_id", "family", "body", "source_refs"] {
		assert!(
			create_schema
				.get("required")
				.and_then(Value::as_array)
				.is_some_and(|fields| { fields.iter().any(|value| value.as_str() == Some(field)) }),
			"Missing Work Journal required field {field}."
		);
	}

	assert!(create_properties.contains_key("write_policy"));
	assert!(create_properties.contains_key("promotion_boundary"));
	assert!(readback_properties.contains_key("session_id"));
	assert!(readback_properties.contains_key("families"));
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

#[test]
fn payload_level_schema_for_search_tools_is_l0_l1_l2() {
	for schema in [
		server::searches_create_schema(),
		server::searches_get_schema(),
		server::searches_timeline_schema(),
		server::searches_notes_schema(),
	] {
		let properties = schema
			.get("properties")
			.and_then(Value::as_object)
			.expect("Search schema is missing properties.");
		let payload_level = properties
			.get("payload_level")
			.and_then(Value::as_object)
			.expect("payload_level field is missing from search schema.");
		let payload_level_values = payload_level
			.get("enum")
			.and_then(Value::as_array)
			.expect("payload_level enum is missing.");

		assert_eq!(payload_level_values.len(), 4, "Unexpected payload_level enum length.");
		assert!(payload_level_values.iter().any(|value| value.as_str() == Some("l0")));
		assert!(payload_level_values.iter().any(|value| value.as_str() == Some("l1")));
		assert!(payload_level_values.iter().any(|value| value.as_str() == Some("l2")));
		assert!(payload_level_values.iter().any(|value| value.is_null()));
	}
}
