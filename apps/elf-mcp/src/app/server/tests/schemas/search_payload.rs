use serde_json::Value;

use crate::app::server;

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
