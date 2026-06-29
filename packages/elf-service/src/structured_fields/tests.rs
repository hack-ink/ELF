use time::OffsetDateTime;

use crate::{
	Error,
	structured_fields::{
		self, StructuredEntity, StructuredFields, StructuredRelation, StructuredRelationObject,
	},
};

fn structured_relation(
	subject: &str,
	predicate: &str,
	object: StructuredRelationObject,
	valid_from: Option<OffsetDateTime>,
	valid_to: Option<OffsetDateTime>,
) -> StructuredFields {
	StructuredFields {
		summary: None,
		facts: None,
		concepts: None,
		entities: None,
		relations: Some(vec![StructuredRelation {
			subject: Some(StructuredEntity {
				canonical: Some(subject.to_string()),
				kind: None,
				aliases: None,
			}),
			predicate: Some(predicate.to_string()),
			object: Some(object),
			valid_from,
			valid_to,
		}]),
	}
}

#[test]
fn fact_binding_accepts_note_text_substring() {
	let structured = StructuredFields {
		summary: None,
		facts: Some(vec!["Deploy uses reranking".to_string()]),
		concepts: None,
		entities: None,
		relations: None,
	};
	let res = structured_fields::validate_structured_fields(
		&structured,
		"Deploy uses reranking after retrieval.",
		&serde_json::json!({}),
		None,
	);

	assert!(res.is_ok());
}

#[test]
fn fact_binding_rejects_without_text_or_evidence() {
	let structured = StructuredFields {
		summary: None,
		facts: Some(vec!["Nonexistent claim.".to_string()]),
		concepts: None,
		entities: None,
		relations: None,
	};
	let res = structured_fields::validate_structured_fields(
		&structured,
		"Some note.",
		&serde_json::json!({}),
		None,
	);

	assert!(res.is_err());
}

#[test]
fn relation_object_requires_exactly_one_of_entity_or_value() {
	let structured = structured_relation(
		"alice",
		"owns",
		StructuredRelationObject {
			entity: Some(StructuredEntity {
				canonical: Some("Acme".to_string()),
				kind: None,
				aliases: None,
			}),
			value: Some("Acme corp".to_string()),
		},
		None,
		None,
	);
	let res = structured_fields::validate_structured_fields(
		&structured,
		"alice owns Acme corp.",
		&serde_json::json!({
			"evidence": [{"quote": "alice owns Acme"}]
		}),
		None,
	);
	let err = res.expect_err("relation should reject object with both entity and value");
	let message = match err {
		Error::InvalidRequest { message } => message,
		_ => panic!("expected invalid request, got {err:?}"),
	};

	assert_eq!(
		message,
		"structured.relations[0].object must provide exactly one of entity or value."
	);
}

#[test]
fn relation_rejects_valid_to_not_after_valid_from() {
	let structured = structured_relation(
		"alice",
		"met",
		StructuredRelationObject { entity: None, value: Some("bob".to_string()) },
		Some(OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("valid timestamp")),
		Some(OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("valid timestamp")),
	);
	let res = structured_fields::validate_structured_fields(
		&structured,
		"alice met bob",
		&serde_json::json!({
			"evidence": [{"quote": "alice met bob"}]
		}),
		None,
	);
	let err = res.expect_err("relation should require valid_to greater than valid_from");
	let message = match err {
		Error::InvalidRequest { message } => message,
		_ => panic!("expected invalid request, got {err:?}"),
	};

	assert_eq!(message, "structured.relations[0].valid_to must be greater than valid_from.");
}

#[test]
fn relation_checks_subject_predicate_and_object_value_are_evidence_bound() {
	let subject_message = match structured_fields::validate_structured_fields(
		&structured_relation(
			"alice",
			"caused",
			StructuredRelationObject { entity: None, value: Some("outage".to_string()) },
			None,
			None,
		),
		"a critical outage was logged.",
		&serde_json::json!({"evidence": [{"quote": "caused an outage"}]}),
		None,
	) {
		Err(Error::InvalidRequest { message }) => message,
		res => panic!("expected invalid request, got {res:?}"),
	};

	assert!(subject_message.contains("structured.relations[0].subject.canonical is not supported"));

	let predicate_message = match structured_fields::validate_structured_fields(
		&structured_relation(
			"operator",
			"discovered",
			StructuredRelationObject { entity: None, value: Some("outage".to_string()) },
			None,
			None,
		),
		"operator monitored a system outage.",
		&serde_json::json!({"evidence": [{"quote": "operator saw outage"}]}),
		None,
	) {
		Err(Error::InvalidRequest { message }) => message,
		res => panic!("expected invalid request, got {res:?}"),
	};

	assert!(predicate_message.contains("structured.relations[0].predicate is not supported"));

	let object_message = match structured_fields::validate_structured_fields(
		&structured_relation(
			"operator",
			"noticed",
			StructuredRelationObject {
				entity: None,
				value: Some("service interruption".to_string()),
			},
			None,
			None,
		),
		"The operator noticed service latency during testing.",
		&serde_json::json!({"evidence": [{"quote": "The operator noticed service behavior"}]}),
		None,
	) {
		Err(Error::InvalidRequest { message }) => message,
		res => panic!("expected invalid request, got {res:?}"),
	};

	assert!(object_message.contains("structured.relations[0].object.value is not supported"));
}

#[test]
fn relation_accepts_valid_structured_relation() {
	let structured = structured_relation(
		"alice",
		"works at",
		StructuredRelationObject {
			entity: Some(StructuredEntity {
				canonical: Some("acme corp".to_string()),
				kind: None,
				aliases: None,
			}),
			value: None,
		},
		Some(OffsetDateTime::from_unix_timestamp(1_699_900_000).expect("valid timestamp")),
		Some(OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("valid timestamp")),
	);
	let res = structured_fields::validate_structured_fields(
		&structured,
		"alice works at acme corp and reported progress.",
		&serde_json::json!({
			"evidence": [{"quote": "works at acme corp"}]
		}),
		None,
	);

	assert!(res.is_ok());
}
