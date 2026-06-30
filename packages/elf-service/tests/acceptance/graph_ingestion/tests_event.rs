use crate::acceptance::graph_ingestion::tests_helpers;
use elf_service::{AddEventRequest, EventMessage, NoteOp};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn add_event_persists_graph_relations() {
	let Some(test_db) = tests_helpers::build_test_db("add_event_persists_graph_relations").await
	else {
		return;
	};
	let extractor_payload = serde_json::json!({
		"notes": [{
			"type": "fact",
			"key": "mentorship",
			"text": "Alice mentors Bob.",
			"structured": {
				"relations": [{
					"subject": { "canonical": "Alice" },
					"predicate": "mentors",
					"object": { "value": "Bob" }
				}]
			},
			"importance": 0.8,
			"confidence": 0.9,
			"ttl_days": null,
			"scope_suggestion": "agent_private",
			"evidence": [{ "message_index": 0, "quote": "Alice mentors Bob." }],
			"reason": "test"
		}]
	});
	let service =
		tests_helpers::build_service_with_extractor_payload(&test_db, extractor_payload).await;

	tests_helpers::reset_service_db(&service).await;

	let response = service
		.add_event(AddEventRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: Some("agent_private".to_string()),
			dry_run: Some(false),
			ingestion_profile: None,
			messages: vec![EventMessage {
				role: "user".to_string(),
				content: "Alice mentors Bob.".to_string(),
				ts: None,
				msg_id: None,
				write_policy: None,
			}],
		})
		.await
		.expect("add_event failed.");

	assert_eq!(response.results.len(), 1);
	assert_eq!(response.results[0].op, NoteOp::Add);

	tests_helpers::assert_graph_policy_from_op(
		response.results[0].op,
		response.results[0].policy_decision,
	);

	let note_id = response.results[0].note_id.expect("Expected note_id.");
	let fact_id = tests_helpers::graph_fact_id(&service.db.pool).await;
	let fact_count = tests_helpers::graph_fact_count(&service.db.pool).await;
	let evidence_count =
		tests_helpers::graph_fact_evidence_count_for_note(&service.db.pool, fact_id, note_id).await;

	assert_eq!(fact_count, 1);
	assert_eq!(evidence_count, 1);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
