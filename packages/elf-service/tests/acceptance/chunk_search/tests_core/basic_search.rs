use uuid::Uuid;

use crate::acceptance::{StubRerank, chunk_search::tests_helpers};
use elf_service::SearchRequest;

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_returns_chunk_items() {
	let providers = tests_helpers::build_providers(StubRerank);
	let Some(context) = tests_helpers::setup_context("search_returns_chunk_items", providers).await
	else {
		return;
	};
	let note_id = Uuid::new_v4();
	let chunk_id = Uuid::new_v4();
	let note_text = "First sentence. Second sentence.";

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

	let response = context
		.service
		.search_raw(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			token_id: None,
			read_profile: "private_only".to_string(),
			payload_level: Default::default(),
			query: "First".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search failed.");
	let item = response.items.first().expect("Expected search result.");

	assert_eq!(item.chunk_id, chunk_id);
	assert!(!item.snippet.is_empty());

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_stitches_adjacent_chunks() {
	let providers = tests_helpers::build_providers(StubRerank);
	let Some(context) =
		tests_helpers::setup_context("search_stitches_adjacent_chunks", providers).await
	else {
		return;
	};
	let note_id = Uuid::new_v4();
	let chunk_texts = ["First sentence. ", "Second sentence. ", "Third sentence."];
	let note_text = chunk_texts.concat();

	tests_helpers::insert_note(
		&context.service.db.pool,
		note_id,
		&note_text,
		&context.embedding_version,
	)
	.await;

	let mut offset = 0_i32;
	let mut chunk_ids = Vec::new();

	for (index, chunk_text) in chunk_texts.iter().enumerate() {
		let chunk_id = Uuid::new_v4();
		let start = offset;
		let end = start + chunk_text.len() as i32;

		tests_helpers::insert_chunk(
			&context.service.db.pool,
			chunk_id,
			note_id,
			index as i32,
			start,
			end,
			chunk_text,
			&context.embedding_version,
		)
		.await;

		chunk_ids.push((chunk_id, start, end, *chunk_text));

		offset = end;
	}

	let (chunk_id, start, end, text) = chunk_ids[1];

	tests_helpers::upsert_point(&context.service, chunk_id, note_id, 1, start, end, text).await;

	let response = context
		.service
		.search_raw(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			token_id: None,
			read_profile: "private_only".to_string(),
			payload_level: Default::default(),
			query: "Second".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search failed.");
	let item = response.items.first().expect("Expected search result.");

	assert_eq!(item.chunk_id, chunk_id);
	assert!(item.snippet.contains("First sentence."));
	assert!(item.snippet.contains("Second sentence."));
	assert!(item.snippet.contains("Third sentence."));

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_skips_missing_chunk_metadata() {
	let providers = tests_helpers::build_providers(StubRerank);
	let Some(context) =
		tests_helpers::setup_context("search_skips_missing_chunk_metadata", providers).await
	else {
		return;
	};
	let note_id = Uuid::new_v4();
	let chunk_id = Uuid::new_v4();
	let note_text = "Missing chunk metadata.";

	tests_helpers::insert_note(
		&context.service.db.pool,
		note_id,
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

	let response = context
		.service
		.search_raw(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			token_id: None,
			read_profile: "private_only".to_string(),
			payload_level: Default::default(),
			query: "Missing".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search failed.");

	assert!(response.items.is_empty());

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}
