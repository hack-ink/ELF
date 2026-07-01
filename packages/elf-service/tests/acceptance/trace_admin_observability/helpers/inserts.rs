use serde_json::Value;
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::acceptance::trace_admin_observability::helpers::{PROJECT_ID, TENANT_ID, TRACE_VERSION};
use elf_service::search::TraceReplayCandidate;

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
