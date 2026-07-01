use crate::acceptance::{
	StubRerank,
	chunk_search::{relation_context::fixture, tests_helpers},
};
use elf_service::{RelationTemporalStatus, SearchRequest};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_raw_quick_marks_historical_relation_context() {
	let providers = tests_helpers::build_providers(StubRerank);
	let Some(context) = fixture::setup_graph_context_test(
		"search_raw_quick_marks_historical_relation_context",
		providers,
		2,
		2,
	)
	.await
	else {
		return;
	};
	let relation_fixture =
		fixture::seed_relation_context_fixture(&context.service, &context.embedding_version).await;
	let response = context
		.service
		.search_raw_quick(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			token_id: None,
			read_profile: "private_only".to_string(),
			payload_level: Default::default(),
			query: "Alice".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search failed.");
	let item = response.items.first().expect("Expected search result.");
	let relation_context = item
		.explain
		.relation_context
		.as_ref()
		.expect("Expected relation context in search explain.");

	assert_eq!(
		relation_context.len(),
		2,
		"Expected current and historical relation facts in context.",
	);
	assert_eq!(relation_context[0].temporal_status, RelationTemporalStatus::Current);

	let historical = relation_context
		.iter()
		.find(|context| context.fact_id == relation_fixture.older_fact_id)
		.expect("Expected historical fact in relation context.");

	assert_eq!(historical.object.value.as_deref(), Some("Bob"));
	assert_eq!(historical.temporal_status, RelationTemporalStatus::Historical);
	assert!(historical.valid_to.is_some());

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}
