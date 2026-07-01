use std::sync::{Arc, atomic::AtomicUsize};

use serde_json::Value;
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::acceptance::{self, SpyExtractor, StubEmbedding, StubRerank};
use elf_service::{
	ElfService, Providers, SearchExplainRequest, TraceGetRequest, TraceRecentListRequest,
	TraceRecentListResponse, TraceTrajectoryGetRequest, search::TraceReplayCandidate,
};
use elf_testkit::TestDatabase;

pub(crate) const TENANT_ID: &str = "tenant_admin_scope";
pub(crate) const PROJECT_ID: &str = "project_admin_scope";
pub(crate) const TRACE_VERSION: i32 = 3;

pub(crate) struct TraceAdminObservabilityFixture {
	pub(crate) service: ElfService,
	pub(crate) test_db: TestDatabase,
}

pub(crate) struct VisibilityTraceFixtureIds {
	pub(crate) trace_one: Uuid,
	pub(crate) trace_two: Uuid,
	pub(crate) trace_three: Uuid,
	pub(crate) item_two: Uuid,
}

pub(crate) async fn setup_service(test_name: &str) -> Option<TraceAdminObservabilityFixture> {
	let Some(test_db) = acceptance::test_db().await else {
		eprintln!("Skipping {test_name}; set ELF_PG_DSN to run this test.");

		return None;
	};
	let Some(qdrant_url) = acceptance::test_qdrant_url() else {
		eprintln!("Skipping {test_name}; set ELF_QDRANT_URL to run this test.");

		return None;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let cfg = acceptance::test_config(
		test_db.dsn().to_string(),
		qdrant_url,
		4_096,
		collection,
		docs_collection,
	);
	let extractor = SpyExtractor {
		calls: Arc::new(AtomicUsize::new(0)),
		payload: serde_json::json!({ "notes": [] }),
	};
	let providers = Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(StubRerank),
		Arc::new(extractor),
	);
	let service =
		acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	Some(TraceAdminObservabilityFixture { service, test_db })
}

pub(crate) async fn insert_trace(
	executor: &PgPool,
	trace_id: Uuid,
	agent_id: &str,
	read_profile: &str,
	query: &str,
	created_at: OffsetDateTime,
) {
	sqlx::query(
		"\
INSERT INTO search_traces (
	trace_id,
	tenant_id,
	project_id,
	agent_id,
	read_profile,
	query,
	expansion_mode,
	expanded_queries,
	allowed_scopes,
	candidate_count,
	top_k,
	config_snapshot,
	trace_version,
	created_at,
	expires_at
)
	VALUES (
		$1,
		$2,
		$3,
		$4,
		$5,
		$6,
		$7,
		$8,
		$9,
		$10,
		$11,
		$12,
		$13,
		$14,
	$15
	)",
	)
	.bind(trace_id)
	.bind(TENANT_ID)
	.bind(PROJECT_ID)
	.bind(agent_id)
	.bind(read_profile)
	.bind(query)
	.bind("full")
	.bind(serde_json::json!([query]))
	.bind(serde_json::json!(["agent_private", "project_shared", "org_shared"]))
	.bind(10_i32)
	.bind(5_i32)
	.bind(serde_json::json!({ "test": true }))
	.bind(TRACE_VERSION)
	.bind(created_at)
	.bind(created_at + Duration::minutes(60))
	.execute(executor)
	.await
	.expect("Failed to insert trace.");
}

pub(crate) async fn insert_trace_item(
	executor: &PgPool,
	item_id: Uuid,
	trace_id: Uuid,
	note_id: Uuid,
	chunk_id: Uuid,
	rank: i32,
) {
	sqlx::query(
		"\
INSERT INTO search_trace_items (
	item_id,
	trace_id,
	note_id,
	chunk_id,
	rank,
	final_score,
	explain
)
VALUES ($1, $2, $3, $4, $5, $6, $7)",
	)
	.bind(item_id)
	.bind(trace_id)
	.bind(note_id)
	.bind(chunk_id)
	.bind(rank)
	.bind(1.0_f32)
	.bind(serde_json::json!({
		"match": { "matched_terms": [], "matched_fields": [] },
		"ranking": {
			"schema": "search_ranking_explain/v2",
			"policy_id": "ranking_v2:test",
			"final_score": 1.0,
			"terms": []
		}
	}))
	.execute(executor)
	.await
	.expect("Failed to insert trace item.");
}

pub(crate) async fn insert_trace_stage(
	executor: &PgPool,
	stage_id: Uuid,
	trace_id: Uuid,
	stage_order: i32,
	stage_name: &str,
	created_at: OffsetDateTime,
) {
	sqlx::query(
		"\
INSERT INTO search_trace_stages (
	stage_id,
	trace_id,
	stage_order,
	stage_name,
	stage_payload,
	created_at
)
VALUES ($1, $2, $3, $4, $5, $6)",
	)
	.bind(stage_id)
	.bind(trace_id)
	.bind(stage_order)
	.bind(stage_name)
	.bind(serde_json::json!({
		"stage_name": stage_name,
		"metrics": { "items": 0 }
	}))
	.bind(created_at)
	.execute(executor)
	.await
	.expect("Failed to insert trace stage.");
}

pub(crate) async fn insert_trace_stage_item(
	executor: &PgPool,
	item_id: Uuid,
	stage_id: Uuid,
	note_id: Uuid,
	chunk_id: Uuid,
	metrics: Value,
) {
	sqlx::query(
		"\
INSERT INTO search_trace_stage_items (
	id,
	stage_id,
	item_id,
	note_id,
	chunk_id,
	metrics
)
VALUES ($1, $2, $3, $4, $5, $6)",
	)
	.bind(Uuid::new_v4())
	.bind(stage_id)
	.bind(item_id)
	.bind(note_id)
	.bind(chunk_id)
	.bind(metrics)
	.execute(executor)
	.await
	.expect("Failed to insert trace stage item.");
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn insert_trace_candidate(
	executor: &PgPool,
	candidate_id: Uuid,
	trace_id: Uuid,
	note_id: Uuid,
	chunk_id: Uuid,
	rank: i32,
	retrieval_rank: i32,
	retrieval_score: f32,
	created_at: OffsetDateTime,
) {
	sqlx::query(
		"\
INSERT INTO search_trace_candidates (
	candidate_id,
	trace_id,
	note_id,
	chunk_id,
	chunk_index,
	snippet,
	candidate_snapshot,
	retrieval_rank,
	rerank_score,
	note_scope,
	note_importance,
	note_updated_at,
	note_hit_count,
	note_last_hit_at,
	created_at,
	expires_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
	)
	.bind(candidate_id)
	.bind(trace_id)
	.bind(note_id)
	.bind(chunk_id)
	.bind(rank)
	.bind("trace candidate snippet")
	.bind({
		let candidate_snapshot = TraceReplayCandidate {
			note_id,
			chunk_id,
			chunk_index: rank,
			snippet: "trace candidate snippet".to_string(),
			retrieval_rank: retrieval_rank as u32,
			retrieval_score: Some(retrieval_score),
			rerank_score: retrieval_score,
			note_scope: "agent_private".to_string(),
			note_importance: 0.6,
			note_updated_at: created_at,
			note_hit_count: 12,
			note_last_hit_at: None,
			diversity_selected: None,
			diversity_selected_rank: None,
			diversity_selected_reason: None,
			diversity_skipped_reason: None,
			diversity_nearest_selected_note_id: None,
			diversity_similarity: None,
			diversity_mmr_score: None,
			diversity_missing_embedding: None,
		};

		serde_json::to_value(candidate_snapshot)
			.expect("Failed to serialize trace replay candidate.")
	})
	.bind(retrieval_rank)
	.bind(retrieval_score)
	.bind("agent_private")
	.bind(0.6_f32)
	.bind(created_at)
	.bind(12_i64)
	.bind(Option::<OffsetDateTime>::None)
	.bind(created_at)
	.bind(created_at + Duration::minutes(90))
	.execute(executor)
	.await
	.expect("Failed to insert trace candidate.");
}

pub(crate) async fn seed_visibility_and_recent_list_traces(
	service: &ElfService,
	now: OffsetDateTime,
) -> VisibilityTraceFixtureIds {
	let trace_one = Uuid::new_v4();
	let trace_two = Uuid::new_v4();
	let trace_three = Uuid::new_v4();
	let item_one = Uuid::new_v4();
	let item_two = Uuid::new_v4();
	let item_three = Uuid::new_v4();
	let note_one = Uuid::new_v4();
	let note_two = Uuid::new_v4();
	let note_three = Uuid::new_v4();
	let chunk_one = Uuid::new_v4();
	let chunk_two = Uuid::new_v4();
	let chunk_three = Uuid::new_v4();

	insert_trace(&service.db.pool, trace_one, "agent_one", "private_only", "one", now).await;
	insert_trace(
		&service.db.pool,
		trace_two,
		"agent_two",
		"private_only",
		"two",
		now - Duration::seconds(10),
	)
	.await;
	insert_trace(
		&service.db.pool,
		trace_three,
		"agent_three",
		"private_only",
		"three",
		now - Duration::seconds(20),
	)
	.await;
	insert_trace_item(&service.db.pool, item_one, trace_one, note_one, chunk_one, 1).await;
	insert_trace_item(&service.db.pool, item_two, trace_two, note_two, chunk_two, 1).await;
	insert_trace_item(&service.db.pool, item_three, trace_three, note_three, chunk_three, 1).await;

	VisibilityTraceFixtureIds { trace_one, trace_two, trace_three, item_two }
}

pub(crate) async fn trace_recent_list_page(
	service: &ElfService,
	cursor_created_at: Option<OffsetDateTime>,
	cursor_trace_id: Option<Uuid>,
) -> TraceRecentListResponse {
	service
		.trace_recent_list(TraceRecentListRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: "admin_agent".to_string(),
			limit: Some(2),
			cursor_created_at,
			cursor_trace_id,
			agent_id_filter: None,
			read_profile: None,
			created_after: None,
			created_before: None,
		})
		.await
		.expect("Failed to list recent traces.")
}

pub(crate) async fn assert_trace_admin_visibility_cross_scope(
	service: &ElfService,
	trace_id: Uuid,
	item_id: Uuid,
) {
	let cross_agent_trace_get = service
		.trace_get(TraceGetRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: "different_agent".to_string(),
			trace_id,
		})
		.await
		.expect("Expected cross-agent trace lookup to bypass agent ownership filtering.");

	assert_eq!(cross_agent_trace_get.trace.trace_id, trace_id);
	assert_eq!(cross_agent_trace_get.trace.agent_id, "agent_two");

	let cross_agent_trajectory = service
		.trace_trajectory_get(TraceTrajectoryGetRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: "different_agent".to_string(),
			trace_id,
		})
		.await
		.expect("Expected cross-agent trajectory lookup to bypass agent ownership filtering.");

	assert_eq!(cross_agent_trajectory.trace.trace_id, trace_id);

	let cross_agent_item = service
		.search_explain(SearchExplainRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: "different_agent".to_string(),
			result_handle: item_id,
		})
		.await
		.expect("Expected cross-agent trace-item lookup to bypass agent ownership filtering.");

	assert_eq!(cross_agent_item.item.result_handle, item_id);
}
