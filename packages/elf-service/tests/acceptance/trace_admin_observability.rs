use serde_json::Value;
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::acceptance::{SpyExtractor, StubEmbedding, StubRerank};
use elf_service::{
	ElfService, SearchExplainRequest, TraceBundleGetRequest, TraceGetRequest,
	TraceRecentListRequest, TraceRecentListResponse, TraceTrajectoryGetRequest,
	search::TraceBundleMode,
};
use elf_testkit::TestDatabase;

const TENANT_ID: &str = "tenant_admin_scope";
const PROJECT_ID: &str = "project_admin_scope";
const TRACE_VERSION: i32 = 3;

struct TraceAdminObservabilityFixture {
	service: ElfService,
	test_db: TestDatabase,
}

struct VisibilityTraceFixtureIds {
	trace_one: Uuid,
	trace_two: Uuid,
	trace_three: Uuid,
	item_two: Uuid,
}

async fn setup_service(test_name: &str) -> Option<TraceAdminObservabilityFixture> {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping {test_name}; set ELF_PG_DSN to run this test.");

		return None;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!("Skipping {test_name}; set ELF_QDRANT_URL to run this test.");

		return None;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let cfg = crate::acceptance::test_config(
		test_db.dsn().to_string(),
		qdrant_url,
		4_096,
		collection,
		docs_collection,
	);
	let extractor = SpyExtractor {
		calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
		payload: serde_json::json!({ "notes": [] }),
	};
	let providers = elf_service::Providers::new(
		std::sync::Arc::new(StubEmbedding { vector_dim: 4_096 }),
		std::sync::Arc::new(StubRerank),
		std::sync::Arc::new(extractor),
	);
	let service =
		crate::acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	crate::acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	Some(TraceAdminObservabilityFixture { service, test_db })
}

async fn insert_trace(
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
\ttrace_id,
\ttenant_id,
\tproject_id,
\tagent_id,
\tread_profile,
\tquery,
\texpansion_mode,
\texpanded_queries,
\tallowed_scopes,
\tcandidate_count,
\ttop_k,
\tconfig_snapshot,
\ttrace_version,
\tcreated_at,
\texpires_at
)
VALUES (
\t$1,
\t$2,
\t$3,
\t$4,
\t$5,
\t$6,
\t$7,
\t$8,
\t$9,
\t$10,
\t$11,
\t$12,
\t$13,
\t$14
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

async fn insert_trace_item(
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
\titem_id,
\ttrace_id,
\tnote_id,
\tchunk_id,
\trank,
\tfinal_score,
\texplain
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

async fn insert_trace_stage(
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
\tstage_id,
\ttrace_id,
\tstage_order,
\tstage_name,
\tstage_payload,
\tcreated_at
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

async fn insert_trace_stage_item(
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
\tid,
\tstage_id,
\titem_id,
\tnote_id,
\tchunk_id,
\tmetrics
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
async fn insert_trace_candidate(
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
\tcandidate_id,
\ttrace_id,
\tnote_id,
\tchunk_id,
\tchunk_index,
\tsnippet,
\tcandidate_snapshot,
\tretrieval_rank,
\trerank_score,
\tnote_scope,
\tnote_importance,
\tnote_updated_at,
\tnote_hit_count,
\tnote_last_hit_at,
\tcreated_at,
\texpires_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
	)
	.bind(candidate_id)
	.bind(trace_id)
	.bind(note_id)
	.bind(chunk_id)
	.bind(rank)
	.bind("trace candidate snippet")
	.bind(serde_json::json!({
		"note_id": note_id,
		"chunk_id": chunk_id,
		"chunk_index": rank,
		"snippet": "trace candidate snippet",
		"retrieval_rank": retrieval_rank,
		"rerank_score": retrieval_score,
		"note_scope": "agent_private",
		"note_importance": 0.6,
		"note_updated_at": created_at,
		"note_hit_count": 12,
		"note_last_hit_at": Option::<OffsetDateTime>::None,
		"diversity_selected": Option::<bool>::None,
		"diversity_selected_rank": Option::<u32>::None,
		"diversity_selected_reason": Option::<String>::None,
		"diversity_skipped_reason": Option::<String>::None,
		"diversity_nearest_selected_note_id": Option::<Uuid>::None,
		"diversity_similarity": Option::<f32>::None,
		"diversity_mmr_score": Option::<f32>::None,
		"diversity_missing_embedding": Option::<bool>::None
	}))
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

async fn seed_visibility_and_recent_list_traces(
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

async fn trace_recent_list_page(
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

async fn assert_trace_admin_visibility_cross_scope(
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
	assert!(candidates[0].rerank_score >= candidates[1].rerank_score);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
