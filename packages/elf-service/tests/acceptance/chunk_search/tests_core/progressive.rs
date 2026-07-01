use uuid::Uuid;

use crate::acceptance::{StubRerank, chunk_search::tests_helpers};
use elf_service::{SearchDetailsRequest, SearchRequest, SearchTimelineRequest};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn progressive_search_returns_index_timeline_and_details() {
	let providers = tests_helpers::build_providers(StubRerank);
	let Some(context) = tests_helpers::setup_context(
		"progressive_search_returns_index_timeline_and_details",
		providers,
	)
	.await
	else {
		return;
	};
	let note_id = Uuid::new_v4();
	let chunk_id = Uuid::new_v4();
	let note_text = "Progressive retrieval works best with staged expansion.";

	tests_helpers::insert_note(
		&context.service.db.pool,
		note_id,
		note_text,
		&context.embedding_version,
	)
	.await;
	tests_helpers::insert_chunk(
		&context.service.db.pool,
		chunk_id,
		note_id,
		0,
		0,
		note_text.len() as i32,
		note_text,
		&context.embedding_version,
	)
	.await;
	tests_helpers::upsert_point(
		&context.service,
		chunk_id,
		note_id,
		0,
		0,
		note_text.len() as i32,
		note_text,
	)
	.await;

	let index = context
		.service
		.search(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			token_id: None,
			read_profile: "private_only".to_string(),
			payload_level: Default::default(),
			query: "Progressive".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search index failed.");

	assert!(!index.items.is_empty());

	let timeline = context
		.service
		.search_timeline(SearchTimelineRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			search_session_id: index.search_session_id,
			payload_level: Default::default(),
			group_by: None,
		})
		.await
		.expect("Search timeline failed.");

	assert!(!timeline.groups.is_empty());

	let details = context
		.service
		.search_details(SearchDetailsRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			search_session_id: index.search_session_id,
			payload_level: Default::default(),
			note_ids: vec![note_id],
			record_hits: Some(false),
		})
		.await
		.expect("Search details failed.");
	let returned = details
		.results
		.first()
		.and_then(|result| result.note.as_ref())
		.expect("Expected note details.");

	assert_eq!(returned.note_id, note_id);
	assert_eq!(returned.text, note_text);

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}
