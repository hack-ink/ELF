use uuid::Uuid;

use crate::acceptance::graph_ingestion::tests_helpers::{
	TEST_PROJECT, TEST_SCOPE, TEST_TENANT, policy,
};
use elf_service::{AddNoteInput, AddNoteRequest, ElfService, NoteOp, StructuredFields};

pub(in crate::acceptance::graph_ingestion) fn fact_note(
	key: &str,
	text: &str,
	predicate: &str,
	object_value: &str,
) -> AddNoteInput {
	let structured = serde_json::from_value::<StructuredFields>(serde_json::json!({
		"relations": [{
			"subject": { "canonical": "Alice" },
			"predicate": predicate,
			"object": { "value": object_value }
		}]
	}))
	.expect("Failed to build structured fields.");

	AddNoteInput {
		r#type: "fact".to_string(),
		key: Some(key.to_string()),
		text: text.to_string(),
		structured: Some(structured),
		importance: 0.8,
		confidence: 0.9,
		ttl_days: None,
		source_ref: serde_json::json!({}),
		write_policy: None,
	}
}

pub(in crate::acceptance::graph_ingestion) fn duplicate_fact_attaches_multiple_evidence_request()
-> AddNoteRequest {
	AddNoteRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![
			AddNoteInput {
				r#type: "fact".to_string(),
				key: Some("mentorship-a".to_string()),
				text: "Alice mentors Bob in 2026.".to_string(),
				structured: Some(
					serde_json::from_value::<elf_service::structured_fields::StructuredFields>(
						serde_json::json!({
							"relations": [{
								"subject": { "canonical": "Alice" },
								"predicate": "mentors",
								"object": { "value": "Bob" }
							}]
						}),
					)
					.expect("Failed to build structured fields."),
				),
				importance: 0.8,
				confidence: 0.9,
				ttl_days: None,
				source_ref: serde_json::json!({}),
				write_policy: None,
			},
			AddNoteInput {
				r#type: "fact".to_string(),
				key: Some("mentorship-b".to_string()),
				text: "Alice also mentors Bob often.".to_string(),
				structured: Some(
					serde_json::from_value::<elf_service::structured_fields::StructuredFields>(
						serde_json::json!({
							"relations": [{
								"subject": { "canonical": "Alice" },
								"predicate": "mentors",
								"object": { "value": "Bob" }
							}]
						}),
					)
					.expect("Failed to build structured fields."),
				),
				importance: 0.7,
				confidence: 0.8,
				ttl_days: None,
				source_ref: serde_json::json!({}),
				write_policy: None,
			},
		],
	}
}

pub(in crate::acceptance::graph_ingestion) async fn add_fact_note(
	service: &ElfService,
	key: &str,
	text: &str,
	predicate: &str,
	object_value: &str,
) -> Uuid {
	let response = service
		.add_note(AddNoteRequest {
			tenant_id: TEST_TENANT.to_string(),
			project_id: TEST_PROJECT.to_string(),
			agent_id: "a".to_string(),
			scope: TEST_SCOPE.to_string(),
			notes: vec![fact_note(key, text, predicate, object_value)],
		})
		.await
		.expect("add_note failed.");

	assert_eq!(response.results.len(), 1);
	assert_eq!(response.results[0].op, NoteOp::Add);

	policy::assert_graph_policy_from_op(
		response.results[0].op,
		response.results[0].policy_decision,
	);

	response.results[0].note_id.expect("Expected note_id.")
}
