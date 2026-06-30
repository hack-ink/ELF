use std::collections::BTreeMap;

use serde_json::Value;
use time::OffsetDateTime;

use crate::{
	recall_debug::{NoteDebugSourceRow, Uuid},
	search::{SearchTrace, SearchTrajectoryStage, TraceReplayCandidate},
};
use elf_storage::models::MemoryNote;

pub(super) fn compact_replay_trace(trace_id: Uuid, now: OffsetDateTime) -> SearchTrace {
	SearchTrace {
		trace_id,
		tenant_id: "tenant".to_string(),
		project_id: "project".to_string(),
		agent_id: "agent".to_string(),
		read_profile: "private_plus_project".to_string(),
		query: "release handoff".to_string(),
		expansion_mode: "dynamic".to_string(),
		expanded_queries: vec!["release handoff".to_string(), "owner transfer".to_string()],
		allowed_scopes: vec!["agent_private".to_string(), "project_shared".to_string()],
		candidate_count: 2,
		top_k: 1,
		config_snapshot: serde_json::json!({
			"search": {
				"expansion": {
					"mode": "dynamic",
					"max_queries": 3,
					"include_original": true
				}
			},
			"ranking": {
				"policy_id": "ranking_v2:test",
				"blend": {
					"enabled": true
				},
				"diversity": {
					"enabled": true
				},
				"retrieval_sources": {
					"fusion_weight": 1.0
				},
				"override": {
					"blend": {
						"enabled": false
					}
				}
			}
		}),
		created_at: now,
		trace_version: 3,
	}
}

pub(super) fn compact_replay_stages() -> Vec<SearchTrajectoryStage> {
	vec![
		SearchTrajectoryStage {
			stage_order: 2,
			stage_name: "recall.candidates".to_string(),
			stage_payload: serde_json::json!({
				"stats": {
					"candidate_count_before_filter": 2,
					"candidate_count_after_filter": 2
				}
			}),
			items: Vec::new(),
		},
		SearchTrajectoryStage {
			stage_order: 4,
			stage_name: "rerank.score".to_string(),
			stage_payload: serde_json::json!({
				"stats": {
					"reranked_count": 2
				},
				"decisions": {
					"blend_enabled": true,
					"diversity_enabled": true
				}
			}),
			items: Vec::new(),
		},
	]
}

pub(super) fn compact_replay_selected_candidate(
	note_id: Uuid,
	chunk_id: Uuid,
	now: OffsetDateTime,
) -> TraceReplayCandidate {
	TraceReplayCandidate {
		note_id,
		chunk_id,
		chunk_index: 0,
		snippet: "selected".to_string(),
		retrieval_rank: 2,
		retrieval_score: Some(0.4),
		rerank_score: 0.9,
		note_scope: "project_shared".to_string(),
		note_importance: 0.7,
		note_updated_at: now,
		note_hit_count: 0,
		note_last_hit_at: None,
		diversity_selected: Some(true),
		diversity_selected_rank: Some(1),
		diversity_selected_reason: Some("mmr".to_string()),
		diversity_skipped_reason: None,
		diversity_nearest_selected_note_id: None,
		diversity_similarity: None,
		diversity_mmr_score: Some(0.8),
		diversity_missing_embedding: Some(false),
	}
}

pub(super) fn compact_replay_dropped_candidate(
	note_id: Uuid,
	chunk_id: Uuid,
	selected_note_id: Uuid,
	now: OffsetDateTime,
) -> TraceReplayCandidate {
	TraceReplayCandidate {
		note_id,
		chunk_id,
		chunk_index: 0,
		snippet: "dropped".to_string(),
		retrieval_rank: 1,
		retrieval_score: Some(0.8),
		rerank_score: 0.1,
		note_scope: "project_shared".to_string(),
		note_importance: 0.3,
		note_updated_at: now,
		note_hit_count: 0,
		note_last_hit_at: None,
		diversity_selected: Some(false),
		diversity_selected_rank: None,
		diversity_selected_reason: None,
		diversity_skipped_reason: Some("not_in_final_top_k".to_string()),
		diversity_nearest_selected_note_id: Some(selected_note_id),
		diversity_similarity: Some(0.92),
		diversity_mmr_score: Some(0.1),
		diversity_missing_embedding: Some(false),
	}
}

pub(super) fn compact_replay_source_refs(
	selected_note_id: Uuid,
	dropped_note_id: Uuid,
	now: OffsetDateTime,
) -> BTreeMap<Uuid, NoteDebugSourceRow> {
	BTreeMap::from([
		(
			selected_note_id,
			NoteDebugSourceRow {
				status: "active".to_string(),
				source_ref: serde_json::json!({"schema": "source_ref/v1", "ref": {"id": "selected"}}),
				updated_at: now,
			},
		),
		(
			dropped_note_id,
			NoteDebugSourceRow {
				status: "active".to_string(),
				source_ref: serde_json::json!({"schema": "source_ref/v1", "ref": {"id": "dropped"}}),
				updated_at: now,
			},
		),
	])
}

pub(super) fn assert_compact_replay_artifact(artifact: &Value) {
	assert_eq!(
		artifact.pointer("/schema").and_then(serde_json::Value::as_str),
		Some("elf.recall_debug.compact_replay/v1")
	);
	assert_eq!(artifact.pointer("/controls/top_k").and_then(serde_json::Value::as_u64), Some(1));
	assert_eq!(
		artifact.pointer("/controls/expanded_query_count").and_then(serde_json::Value::as_u64),
		Some(2)
	);
	assert_eq!(
		artifact.pointer("/controls/ranking/policy_id").and_then(serde_json::Value::as_str),
		Some("ranking_v2:test")
	);
	assert_eq!(
		artifact.pointer("/stage_movement/1/stage_name").and_then(serde_json::Value::as_str),
		Some("rerank.score")
	);
	assert_eq!(
		artifact.pointer("/candidate_replay/selected_count").and_then(serde_json::Value::as_u64),
		Some(1)
	);
	assert_eq!(
		artifact
			.pointer("/candidate_replay/rows/0/selection_state")
			.and_then(serde_json::Value::as_str),
		Some("selected")
	);
	assert_eq!(
		artifact
			.pointer("/candidate_replay/rows/0/source_ref_available")
			.and_then(serde_json::Value::as_bool),
		Some(true)
	);
	assert_eq!(
		artifact
			.pointer("/candidate_replay/rows/0/rerank_delta")
			.and_then(serde_json::Value::as_i64),
		Some(1)
	);
	assert_eq!(
		artifact
			.pointer("/candidate_replay/rows/0/policy_reason")
			.and_then(serde_json::Value::as_str),
		Some("mmr")
	);
	assert_eq!(
		artifact
			.pointer("/candidate_replay/rows/1/selection_state")
			.and_then(serde_json::Value::as_str),
		Some("dropped")
	);
	assert_eq!(
		artifact.pointer("/authority/raw_sql_needed").and_then(serde_json::Value::as_bool),
		Some(false)
	);
}

pub(super) fn note_for_debug_visibility(agent_id: &str, scope: &str, status: &str) -> MemoryNote {
	let now = OffsetDateTime::now_utc();

	MemoryNote {
		note_id: Uuid::new_v4(),
		tenant_id: "tenant-a".to_string(),
		project_id: "project-a".to_string(),
		agent_id: agent_id.to_string(),
		scope: scope.to_string(),
		r#type: "fact".to_string(),
		key: None,
		text: "Fact: debug visibility test note.".to_string(),
		importance: 0.7,
		confidence: 0.9,
		status: status.to_string(),
		created_at: now,
		updated_at: now,
		expires_at: None,
		embedding_version: "test:v1".to_string(),
		source_ref: serde_json::json!({"schema": "source_ref/v1"}),
		hit_count: 0,
		last_hit_at: None,
	}
}
