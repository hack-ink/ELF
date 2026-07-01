use uuid::Uuid;

use crate::acceptance::structured_field_retrieval::support::{self, TestContext, UpsertPointArgs};
use elf_service::SearchRequest;

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn structured_fact_field_can_surface_note_and_marks_matched_fields() {
	let Some(context) =
		support::setup_context("structured_fact_field_can_surface_note_and_marks_matched_fields")
			.await
	else {
		return;
	};
	let query = "alpha unique";

	insert_confuser_notes(&context, query).await;

	let structured_note_id = insert_structured_fact_note(&context, query).await;
	let response = context
		.service
		.search_raw(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			token_id: None,
			read_profile: "private_only".to_string(),
			payload_level: Default::default(),
			query: query.to_string(),
			top_k: Some(1),
			candidate_k: Some(10),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search failed.");
	let item = response.items.first().expect("Expected search result.");

	assert_eq!(item.note_id, structured_note_id);
	assert!(
		item.explain.r#match.matched_fields.iter().any(|field| field == "facts"),
		"Expected matched_fields to include facts; got {:?}",
		item.explain.r#match.matched_fields
	);

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}

async fn insert_confuser_notes(context: &TestContext, query: &str) {
	for i in 0..20 {
		let note_id = Uuid::new_v4();
		let chunk_id = Uuid::new_v4();
		let text = format!("Confuser {i}: {query}.");

		support::insert_note(&context.service.db.pool, note_id, &text, &context.embedding_version)
			.await;
		support::insert_chunk(
			&context.service.db.pool,
			chunk_id,
			note_id,
			0,
			0,
			text.len() as i32,
			&text,
			&context.embedding_version,
		)
		.await;
		support::upsert_point(
			&context.service,
			UpsertPointArgs {
				chunk_id,
				note_id,
				chunk_index: 0,
				start_offset: 0,
				end_offset: text.len() as i32,
				text: &text,
				dense: vec![0.0_f32; 4_096],
			},
		)
		.await;
	}
}

async fn insert_structured_fact_note(context: &TestContext, query: &str) -> Uuid {
	let structured_note_id = Uuid::new_v4();
	let structured_chunk_id = Uuid::new_v4();
	let structured_chunk_text = "ZEBRA chunk text does not include the query.";

	support::insert_note(
		&context.service.db.pool,
		structured_note_id,
		"This note is generic.",
		&context.embedding_version,
	)
	.await;
	support::insert_chunk(
		&context.service.db.pool,
		structured_chunk_id,
		structured_note_id,
		0,
		0,
		structured_chunk_text.len() as i32,
		structured_chunk_text,
		&context.embedding_version,
	)
	.await;
	support::insert_chunk_embedding(
		&context.service.db.pool,
		structured_chunk_id,
		&context.embedding_version,
	)
	.await;
	support::upsert_point(
		&context.service,
		UpsertPointArgs {
			chunk_id: structured_chunk_id,
			note_id: structured_note_id,
			chunk_index: 0,
			start_offset: 0,
			end_offset: structured_chunk_text.len() as i32,
			text: structured_chunk_text,
			dense: vec![1.0_f32; 4_096],
		},
	)
	.await;

	let field_id = Uuid::new_v4();

	support::insert_fact_field_row(&context.service.db.pool, field_id, structured_note_id, query)
		.await;
	support::insert_fact_field_embedding(
		&context.service.db.pool,
		field_id,
		&context.embedding_version,
	)
	.await;

	structured_note_id
}
