use serde_json::Value;

use crate::app::server;

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
