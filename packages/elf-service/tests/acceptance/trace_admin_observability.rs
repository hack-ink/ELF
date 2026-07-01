mod helpers;

pub(crate) use helpers::{
	PROJECT_ID, TENANT_ID, TraceAdminObservabilityFixture, VisibilityTraceFixtureIds,
	assertions::assert_trace_admin_visibility_cross_scope,
	inserts::{
		insert_trace, insert_trace_candidate, insert_trace_item, insert_trace_stage,
		insert_trace_stage_item,
	},
	seed::{seed_visibility_and_recent_list_traces, trace_recent_list_page},
	setup::setup_service,
};

use time::OffsetDateTime;
use uuid::Uuid;

use elf_service::{TraceBundleGetRequest, search::TraceBundleMode};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn trace_admin_visibility_and_recent_list_cursor() {
	let Some(fixture) = setup_service("trace_admin_visibility_and_recent_list_cursor").await else {
		return;
	};
	let TraceAdminObservabilityFixture { service, test_db } = fixture;
	let now = OffsetDateTime::now_utc();
	let VisibilityTraceFixtureIds { trace_one, trace_two, trace_three, item_two } =
		seed_visibility_and_recent_list_traces(&service, now).await;
	let first = trace_recent_list_page(&service, None, None).await;

	assert_eq!(first.schema, "elf.recent_traces/v1");
	assert_eq!(first.traces.len(), 2);
	assert_eq!(first.traces[0].trace_id, trace_one);
	assert_eq!(first.traces[1].trace_id, trace_two);
	assert!(first.traces[0].created_at > first.traces[1].created_at);

	let Some(cursor) = first.next_cursor else {
		panic!("Expected next_cursor to exist for second page.");
	};
	let second =
		trace_recent_list_page(&service, Some(cursor.created_at), Some(cursor.trace_id)).await;

	assert_eq!(second.traces.len(), 1);
	assert_eq!(second.traces[0].trace_id, trace_three);
	assert!(second.next_cursor.is_none());

	assert_trace_admin_visibility_cross_scope(&service, trace_two, item_two).await;

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn trace_bundle_truncation_and_candidate_limits() {
	let Some(fixture) = setup_service("trace_bundle_truncation_and_candidate_limits").await else {
		return;
	};
	let TraceAdminObservabilityFixture { service, test_db } = fixture;
	let now = OffsetDateTime::now_utc();
	let trace_id = Uuid::new_v4();
	let stage_id = Uuid::new_v4();

	insert_trace(&service.db.pool, trace_id, "agent_one", "private_only", "bundle", now).await;
	insert_trace_stage(&service.db.pool, stage_id, trace_id, 0, "selection.final", now).await;

	for index in 0..3 {
		let item_id = Uuid::new_v4();
		let note_id = Uuid::new_v4();
		let chunk_id = Uuid::new_v4();

		insert_trace_item(&service.db.pool, item_id, trace_id, note_id, chunk_id, index + 1).await;
		insert_trace_stage_item(
			&service.db.pool,
			item_id,
			stage_id,
			note_id,
			chunk_id,
			serde_json::json!({ "candidate_index": index }),
		)
		.await;
	}
	for (idx, rank) in [(2_i32, 2_i32), (1_i32, 1_i32), (3_i32, 3_i32)] {
		insert_trace_candidate(
			&service.db.pool,
			Uuid::new_v4(),
			trace_id,
			Uuid::new_v4(),
			Uuid::new_v4(),
			idx,
			rank,
			0.9_f32 - (idx as f32 * 0.1),
			now,
		)
		.await;
	}

	let bounded = service
		.trace_bundle_get(TraceBundleGetRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: "admin_agent".to_string(),
			trace_id,
			mode: TraceBundleMode::Bounded,
			stage_items_limit: Some(1),
			candidates_limit: None,
		})
		.await
		.expect("Failed to fetch bounded bundle.");

	assert_eq!(bounded.schema, "elf.trace_bundle/v1");
	assert_eq!(bounded.stages.len(), 1);
	assert_eq!(bounded.stages[0].items.len(), 1);
	assert!(bounded.candidates.is_none());

	let full = service
		.trace_bundle_get(TraceBundleGetRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: "admin_agent".to_string(),
			trace_id,
			mode: TraceBundleMode::Full,
			stage_items_limit: Some(1),
			candidates_limit: Some(2),
		})
		.await
		.expect("Failed to fetch full bundle.");

	assert_eq!(full.stages[0].items.len(), 1);
	assert!(full.candidates.as_ref().is_some_and(|candidates| candidates.len() == 2));

	let candidates = full.candidates.unwrap();

	assert_eq!(candidates[0].retrieval_rank, 1);
	assert_eq!(candidates[1].retrieval_rank, 2);
	assert!(
		candidates[0].retrieval_score.is_some_and(|score| (score - 0.8_f32).abs() < 1e-6),
		"Unexpected retrieval_score: {:?}",
		candidates[0].retrieval_score
	);
	assert!(candidates[0].rerank_score >= candidates[1].rerank_score);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
