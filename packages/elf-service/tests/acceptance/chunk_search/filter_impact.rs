use serde_json::Value;
use uuid::Uuid;

use crate::acceptance::{
	StubRerank,
	chunk_search::tests_helpers::{self, TestContext},
};
use elf_service::{SearchRequest, TraceTrajectoryGetRequest};

async fn seed_filter_impact_notes(
	context: &TestContext,
	low_note_id: Uuid,
	high_note_id: Uuid,
	low_chunk_id: Uuid,
	high_chunk_id: Uuid,
	low_note_text: &str,
	high_note_text: &str,
) {
	tests_helpers::insert_note_with_importance(
		&context.service.db.pool,
		low_note_id,
		low_note_text,
		&context.embedding_version,
		0.2,
		0.2,
		"agent_private",
	)
	.await;
	tests_helpers::insert_note_with_importance(
		&context.service.db.pool,
		high_note_id,
		high_note_text,
		&context.embedding_version,
		0.9,
		0.9,
		"agent_private",
	)
	.await;
	tests_helpers::insert_chunk(
		&context.service.db.pool,
		low_chunk_id,
		low_note_id,
		0,
		0,
		low_note_text.len() as i32,
		low_note_text,
		&context.embedding_version,
	)
	.await;
	tests_helpers::insert_chunk(
		&context.service.db.pool,
		high_chunk_id,
		high_note_id,
		0,
		0,
		high_note_text.len() as i32,
		high_note_text,
		&context.embedding_version,
	)
	.await;
	tests_helpers::upsert_point(
		&context.service,
		low_chunk_id,
		low_note_id,
		0,
		0,
		low_note_text.len() as i32,
		low_note_text,
	)
	.await;
	tests_helpers::upsert_point(
		&context.service,
		high_chunk_id,
		high_note_id,
		0,
		0,
		high_note_text.len() as i32,
		high_note_text,
	)
	.await;
}

async fn load_filter_impact_from_trace(context: &TestContext, trace_id: Uuid) -> Value {
	let trajectory = context
		.service
		.trace_trajectory_get(TraceTrajectoryGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			trace_id,
		})
		.await
		.expect("Failed to fetch trace trajectory.");

	trajectory
		.stages
		.iter()
		.find(|stage| stage.stage_name == "recall.candidates")
		.expect("Expected recall.candidates stage.")
		.stage_payload
		.get("filter_impact")
		.expect("Expected filter_impact in recall stage.")
		.clone()
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run this test."]
async fn search_filter_affects_candidate_set_and_records_filter_impact() {
	let provider = tests_helpers::build_providers(StubRerank);
	let low_note_text = "alpha low confidence note";
	let high_note_text = "alpha high confidence note";
	let low_note_id = Uuid::new_v4();
	let high_note_id = Uuid::new_v4();
	let low_chunk_id = Uuid::new_v4();
	let high_chunk_id = Uuid::new_v4();
	let mut context = match tests_helpers::setup_context(
		"search_filter_affects_candidate_set_and_records_filter_impact",
		provider,
	)
	.await
	{
		Some(context) => context,
		None => return,
	};

	context.service.cfg.search.explain.write_mode = "inline".to_string();

	seed_filter_impact_notes(
		&context,
		low_note_id,
		high_note_id,
		low_chunk_id,
		high_chunk_id,
		low_note_text,
		high_note_text,
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
			query: "alpha".to_string(),
			top_k: Some(1),
			candidate_k: Some(10),
			filter: Some(serde_json::json!({
				"schema": "search_filter_expr/v1",
				"expr": { "op": "gte", "field": "importance", "value": 0.5 },
			})),
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Search failed.");

	assert_eq!(response.items.len(), 1);
	assert_eq!(response.items[0].note_id, high_note_id);

	let filter_impact = load_filter_impact_from_trace(&context, response.trace_id).await;
	let filter = filter_impact.get("filter").expect("Expected filter object in filter_impact.");
	let requested_candidate_k = filter_impact
		.get("requested_candidate_k")
		.and_then(Value::as_u64)
		.expect("Expected requested_candidate_k.");
	let effective_candidate_k = filter_impact
		.get("effective_candidate_k")
		.and_then(Value::as_u64)
		.expect("Expected effective_candidate_k.");

	assert_eq!(
		filter_impact.get("schema"),
		Some(&Value::String("search_filter_impact/v1".to_string()))
	);
	assert_eq!(requested_candidate_k, 10);
	assert_eq!(effective_candidate_k, 30);
	assert_eq!(filter.get("schema"), Some(&Value::String("search_filter_expr/v1".to_string())));
	assert_eq!(filter_impact.get("candidate_count_pre"), Some(&Value::from(2_u64)));
	assert_eq!(filter_impact.get("candidate_count_post"), Some(&Value::from(1_u64)));
	assert_eq!(filter_impact.get("dropped_total"), Some(&Value::from(1_u64)));

	context.test_db.cleanup().await.expect("Failed to cleanup test database.");
}
