use serde_json::Value;
use uuid::Uuid;

use crate::acceptance::{
	StubRerank,
	chunk_search::{self, TestContext},
};
use elf_service::{NoteFetchResponse, PayloadLevel, SearchDetailsRequest, SearchRequest};

fn build_payload_shape_search_request(payload_level: PayloadLevel) -> SearchRequest {
	SearchRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		token_id: None,
		read_profile: "private_only".to_string(),
		payload_level,
		query: "payload".to_string(),
		top_k: Some(5),
		candidate_k: Some(10),
		filter: None,
		record_hits: Some(false),
		ranking: None,
	}
}

fn assert_search_detail_payload_levels(
	max_note_chars: usize,
	note_text: &str,
	source_ref: &Value,
	l0: &NoteFetchResponse,
	l1: &NoteFetchResponse,
	l2: &NoteFetchResponse,
) {
	assert!(l0.text.chars().count() <= max_note_chars + 3);
	assert!(l1.text.chars().count() <= max_note_chars + 3);
	assert!(l0.text.ends_with("..."));
	assert_eq!(l2.text, note_text);
	assert_ne!(l0.text, l1.text);
	assert_ne!(l0.text, note_text);
	assert_ne!(l1.text, note_text);
	assert!(l1.text.contains("Structured summary"));
	assert_eq!(l0.source_ref, serde_json::json!({}));
	assert_eq!(l1.source_ref, serde_json::json!({}));
	assert_eq!(l2.source_ref, *source_ref);
	assert!(l0.structured.is_none());
	assert!(l1.structured.is_some());
	assert!(l2.structured.is_some());
}

async fn fetch_raw_source_ref_for_level(
	context: &TestContext,
	note_id: Uuid,
	payload_level: PayloadLevel,
) -> Value {
	let response = context
		.service
		.search_raw(build_payload_shape_search_request(payload_level))
		.await
		.expect("Search failed.");
	let item = response.items.first().expect("Expected search result.");

	assert_eq!(item.note_id, note_id);

	item.source_ref.clone()
}

async fn fetch_search_detail_note_for_level(
	context: &TestContext,
	search_session_id: Uuid,
	note_id: Uuid,
	payload_level: PayloadLevel,
) -> NoteFetchResponse {
	let response = context
		.service
		.search_details(SearchDetailsRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			search_session_id,
			payload_level,
			note_ids: vec![note_id],
			record_hits: Some(false),
		})
		.await
		.expect("Search details failed.");

	response
		.results
		.first()
		.and_then(|item| item.note.as_ref())
		.expect("Expected note details.")
		.clone()
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_raw_payload_level_shapes_source_ref() {
	let providers = chunk_search::build_providers(StubRerank);
	let Some(context) =
		chunk_search::setup_context("search_raw_payload_level_shapes_source_ref", providers).await
	else {
		return;
	};
	let note_id = Uuid::new_v4();
	let chunk_id = Uuid::new_v4();
	let note_text = "Payload shaping should control the raw item source_ref payload.";
	let source_ref = serde_json::json!({
		"schema": "note_source_ref/v1",
		"locator": {
			"doc_id": Uuid::new_v4().to_string(),
			"chunk_id": Uuid::new_v4().to_string()
		},
		"metadata": {
			"long_field": "A long metadata body to represent a heavy source reference shape."
		}
	});

	chunk_search::insert_note_with_importance_and_source_ref(
		&context.service.db.pool,
		note_id,
		note_text,
		&context.embedding_version,
		0.9_f32,
		1.0,
		"agent_private",
		source_ref.clone(),
	)
	.await;
	chunk_search::insert_chunk(
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
	chunk_search::upsert_point(
		&context.service,
		chunk_id,
		note_id,
		0,
		0,
		note_text.len() as i32,
		note_text,
	)
	.await;

	let l0 = fetch_raw_source_ref_for_level(&context, note_id, PayloadLevel::L0).await;
	let l1 = fetch_raw_source_ref_for_level(&context, note_id, PayloadLevel::L1).await;
	let l2 = fetch_raw_source_ref_for_level(&context, note_id, PayloadLevel::L2).await;

	assert_eq!(l0, serde_json::json!({}));
	assert_eq!(l1, serde_json::json!({}));
	assert_eq!(l2, source_ref);

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn search_details_payload_level_shapes_text_and_fields() {
	let providers = chunk_search::build_providers(StubRerank);
	let Some(context) = chunk_search::setup_context(
		"search_details_payload_level_shapes_text_and_fields",
		providers,
	)
	.await
	else {
		return;
	};
	let note_id = Uuid::new_v4();
	let chunk_id = Uuid::new_v4();
	let max_note_chars = context.service.cfg.memory.max_note_chars as usize;
	let note_text_seed =
		"This is the long note body used for detail shaping and payload truncation. ";
	let note_text = note_text_seed.repeat((max_note_chars / note_text_seed.len()) + 2);
	let source_ref = serde_json::json!({
		"schema": "note_source_ref/v1",
		"locator": {
			"document_id": Uuid::new_v4().to_string(),
			"chunk_id": Uuid::new_v4().to_string(),
			"extra": "field with rich details for l2 retention"
		},
	});
	let structured_summary = "Structured summary about payload levels and compact text behavior.";
	let field_id = Uuid::new_v4();

	assert!(note_text.len() > max_note_chars);

	chunk_search::insert_note_with_importance_and_source_ref(
		&context.service.db.pool,
		note_id,
		note_text.as_str(),
		&context.embedding_version,
		0.8_f32,
		1.0,
		"agent_private",
		source_ref.clone(),
	)
	.await;
	chunk_search::insert_summary_field_row(
		&context.service.db.pool,
		field_id,
		note_id,
		structured_summary,
	)
	.await;
	chunk_search::insert_chunk(
		&context.service.db.pool,
		chunk_id,
		note_id,
		0,
		0,
		note_text.len() as i32,
		note_text.as_str(),
		&context.embedding_version,
	)
	.await;
	chunk_search::upsert_point(
		&context.service,
		chunk_id,
		note_id,
		0,
		0,
		note_text.len() as i32,
		note_text.as_str(),
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
			payload_level: PayloadLevel::L2,
			query: "payload".to_string(),
			top_k: Some(5),
			candidate_k: Some(10),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search index failed.");
	let l0 = fetch_search_detail_note_for_level(
		&context,
		index.search_session_id,
		note_id,
		PayloadLevel::L0,
	)
	.await;
	let l1 = fetch_search_detail_note_for_level(
		&context,
		index.search_session_id,
		note_id,
		PayloadLevel::L1,
	)
	.await;
	let l2 = fetch_search_detail_note_for_level(
		&context,
		index.search_session_id,
		note_id,
		PayloadLevel::L2,
	)
	.await;

	assert_search_detail_payload_levels(max_note_chars, &note_text, &source_ref, &l0, &l1, &l2);

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}
