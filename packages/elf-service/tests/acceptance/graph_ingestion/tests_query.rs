use crate::acceptance::graph_ingestion::tests_helpers::{
	self, TEST_PROJECT, TEST_SCOPE, TEST_TENANT,
};
use elf_service::{
	DeleteRequest, GraphQueryEntityRef, GraphQueryPredicateRef, GraphQueryRequest, NoteOp,
};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn graph_query_suppresses_deleted_evidence_notes() {
	let Some(test_db) =
		tests_helpers::build_test_db("graph_query_suppresses_deleted_evidence_notes").await
	else {
		return;
	};
	let service = tests_helpers::build_stub_service(&test_db).await;

	tests_helpers::reset_service_db(&service).await;

	let note_id = tests_helpers::add_fact_note(
		&service,
		"mentorship",
		"Alice mentors Bob.",
		"mentors",
		"Bob",
	)
	.await;
	let before_delete = service
		.graph_query(GraphQueryRequest {
			tenant_id: TEST_TENANT.to_string(),
			project_id: TEST_PROJECT.to_string(),
			agent_id: "a".to_string(),
			read_profile: "private_only".to_string(),
			subject: GraphQueryEntityRef::Surface { surface: "Alice".to_string() },
			predicate: Some(GraphQueryPredicateRef::Surface { surface: "mentors".to_string() }),
			scopes: Some(vec![TEST_SCOPE.to_string()]),
			as_of: None,
			limit: Some(10),
			explain: Some(true),
		})
		.await
		.expect("graph query before delete should succeed");

	assert_eq!(before_delete.facts.len(), 1);
	assert_eq!(before_delete.facts[0].evidence_note_ids, vec![note_id]);

	let delete = service
		.delete(DeleteRequest {
			tenant_id: TEST_TENANT.to_string(),
			project_id: TEST_PROJECT.to_string(),
			agent_id: "a".to_string(),
			note_id,
		})
		.await
		.expect("note delete should succeed");

	assert_eq!(delete.op, NoteOp::Delete);

	let after_delete = service
		.graph_query(GraphQueryRequest {
			tenant_id: TEST_TENANT.to_string(),
			project_id: TEST_PROJECT.to_string(),
			agent_id: "a".to_string(),
			read_profile: "private_only".to_string(),
			subject: GraphQueryEntityRef::Surface { surface: "Alice".to_string() },
			predicate: Some(GraphQueryPredicateRef::Surface { surface: "mentors".to_string() }),
			scopes: Some(vec![TEST_SCOPE.to_string()]),
			as_of: None,
			limit: Some(10),
			explain: Some(true),
		})
		.await
		.expect("graph query after delete should succeed");

	assert!(
		after_delete.facts.is_empty(),
		"graph facts without active readable evidence notes must be suppressed"
	);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
