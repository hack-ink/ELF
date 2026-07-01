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
