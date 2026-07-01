use uuid::Uuid;

use crate::acceptance::chunk_search::tests_helpers::{self, KeywordRerank};
use elf_service::SearchRequest;

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_dedupes_note_results() {
	let providers = tests_helpers::build_providers(KeywordRerank { keyword: "preferred" });
	let Some(context) =
		tests_helpers::setup_context("search_dedupes_note_results", providers).await
	else {
		return;
	};
	let note_id = Uuid::new_v4();
	let chunk_texts = ["preferred alpha. ", "bridge chunk. ", "other alpha."];
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

	let (chunk_id_a, start_a, end_a, text_a) = chunk_ids[0];
	let (chunk_id_c, start_c, end_c, text_c) = chunk_ids[2];

	tests_helpers::upsert_point(&context.service, chunk_id_a, note_id, 0, start_a, end_a, text_a)
		.await;
	tests_helpers::upsert_point(&context.service, chunk_id_c, note_id, 2, start_c, end_c, text_c)
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
			query: "alpha".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search failed.");
	let item = response.items.first().expect("Expected search result.");

	assert_eq!(response.items.len(), 1);
	assert_eq!(item.note_id, note_id);
	assert!(
		item.chunk_id == chunk_id_a || item.chunk_id == chunk_id_c,
		"Expected deduped result chunk_id to be one of the ingested chunks."
	);

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}
