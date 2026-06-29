use std::collections::{BTreeMap, HashSet};

use time::OffsetDateTime;

use crate::{
	RecallDebugRow,
	access::SharedSpaceGrantKey,
	recall_debug::{self, BTreeSet, Error, NoteDebugSourceRow, Uuid},
	search::{SearchTrace, SearchTrajectoryStage, TraceReplayCandidate},
};
use elf_storage::models::MemoryNote;

#[test]
fn summary_preserves_not_requested_and_replay_counts() {
	let layers = vec![
		recall_debug::not_requested_layer("graph_facts", "missing graph subject"),
		recall_debug::layer_from_rows(
			"memory_notes",
			"pass",
			Some("trace".to_string()),
			"trace rows",
			vec![
				RecallDebugRow {
					layer: "memory_notes".to_string(),
					item_ref: serde_json::json!({"note_id": "n1"}),
					selection_state: "selected".to_string(),
					authority_layer: "memory_note".to_string(),
					freshness_state: "active".to_string(),
					source_refs: serde_json::json!([]),
					score: Some(1.0),
					rank: Some(1),
					rationale: None,
					stage_reason: None,
					replay_command: Some("elf_admin_trace_bundle_get".to_string()),
					evidence_class: "pass".to_string(),
					debug_artifacts: serde_json::json!({}),
				},
				RecallDebugRow {
					layer: "memory_notes".to_string(),
					item_ref: serde_json::json!({"note_id": "n2"}),
					selection_state: "dropped".to_string(),
					authority_layer: "memory_note".to_string(),
					freshness_state: "active".to_string(),
					source_refs: serde_json::json!([]),
					score: Some(0.5),
					rank: Some(2),
					rationale: None,
					stage_reason: Some("not_in_final_top_k".to_string()),
					replay_command: Some("elf_admin_trace_bundle_get".to_string()),
					evidence_class: "pass".to_string(),
					debug_artifacts: serde_json::json!({}),
				},
			],
		),
	];
	let summary = recall_debug::summarize_layers(&layers);

	assert_eq!(summary.layer_count, 2);
	assert_eq!(summary.row_count, 2);
	assert_eq!(summary.selected_count, 1);
	assert_eq!(summary.dropped_count, 1);
	assert_eq!(summary.not_requested_layer_count, 1);
	assert_eq!(summary.replay_command_count, 2);
	assert_eq!(summary.evidence_class_counts.get("pass"), Some(&1));
	assert_eq!(summary.evidence_class_counts.get("not_requested"), Some(&1));
}

#[test]
fn not_requested_layers_never_require_raw_sql() {
	let layer = recall_debug::not_requested_layer("source_documents", "missing query");

	assert_eq!(layer.evidence_class, "not_requested");
	assert_eq!(layer.row_count, 0);
	assert!(!layer.raw_sql_needed);
	assert!(!layer.replayable);
}

#[test]
fn blocked_layers_are_counted_as_incomplete_evidence() {
	let layer = recall_debug::blocked_layer(
		"source_documents",
		Some("alpha".to_string()),
		"docs search failed",
		&Error::Storage { message: "database unavailable".to_string() },
	);
	let summary = recall_debug::summarize_layers(&[layer]);

	assert_eq!(summary.layer_count, 1);
	assert_eq!(summary.incomplete_layer_count, 1);
	assert_eq!(summary.evidence_class_counts.get("blocked"), Some(&1));
}

#[test]
fn blocked_layer_does_not_expose_raw_backend_errors() {
	let layer = recall_debug::blocked_layer(
		"graph_facts",
		None,
		"graph report failed",
		&Error::Storage { message: "password=secret host=db.internal".to_string() },
	);

	assert!(layer.summary.contains("error_class=storage_unavailable"));
	assert!(!layer.summary.contains("password=secret"));
	assert!(!layer.summary.contains("db.internal"));
}

#[test]
fn selected_candidate_filter_is_chunk_level() {
	let note_id = Uuid::new_v4();
	let selected_chunk_id = Uuid::new_v4();
	let dropped_chunk_id = Uuid::new_v4();
	let selected = BTreeSet::from([recall_debug::candidate_identity(note_id, selected_chunk_id)]);

	assert!(selected.contains(&recall_debug::candidate_identity(note_id, selected_chunk_id)));
	assert!(!selected.contains(&recall_debug::candidate_identity(note_id, dropped_chunk_id)));
}

fn compact_replay_trace(trace_id: Uuid, now: OffsetDateTime) -> SearchTrace {
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

fn compact_replay_stages() -> Vec<SearchTrajectoryStage> {
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

fn compact_replay_selected_candidate(
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

fn compact_replay_dropped_candidate(
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

fn compact_replay_source_refs(
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

fn assert_compact_replay_artifact(artifact: &serde_json::Value) {
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

#[test]
fn compact_replay_artifact_exposes_controls_stage_movement_and_rerank_effects() {
	let trace_id = Uuid::new_v4();
	let selected_note_id = Uuid::new_v4();
	let selected_chunk_id = Uuid::new_v4();
	let dropped_note_id = Uuid::new_v4();
	let dropped_chunk_id = Uuid::new_v4();
	let now = OffsetDateTime::from_unix_timestamp(0).expect("Valid timestamp.");
	let candidates = vec![
		compact_replay_selected_candidate(selected_note_id, selected_chunk_id, now),
		compact_replay_dropped_candidate(dropped_note_id, dropped_chunk_id, selected_note_id, now),
	];
	let selected =
		BTreeSet::from([recall_debug::candidate_identity(selected_note_id, selected_chunk_id)]);
	let source_refs = compact_replay_source_refs(selected_note_id, dropped_note_id, now);
	let artifact = recall_debug::memory_compact_replay_artifact(
		&compact_replay_trace(trace_id, now),
		compact_replay_stages().as_slice(),
		candidates.as_slice(),
		&[],
		&selected,
		&source_refs,
		"elf_admin_trace_bundle_get trace_id=<trace> mode=bounded",
	);

	assert_compact_replay_artifact(&artifact);
}

#[test]
fn debug_note_readability_requires_current_note_and_scope_access() {
	let allowed_scopes = vec!["agent_private".to_string(), "project_shared".to_string()];
	let shared_grants = HashSet::new();
	let now = OffsetDateTime::now_utc();
	let mut note = note_for_debug_visibility("owner-agent", "agent_private", "active");

	assert!(recall_debug::note_debug_read_allowed(
		&note,
		"owner-agent",
		&allowed_scopes,
		&shared_grants,
		now
	));
	assert!(!recall_debug::note_debug_read_allowed(
		&note,
		"other-agent",
		&allowed_scopes,
		&shared_grants,
		now
	));

	note.status = "deleted".to_string();

	assert!(!recall_debug::note_debug_read_allowed(
		&note,
		"owner-agent",
		&allowed_scopes,
		&shared_grants,
		now
	));

	note.status = "deprecated".to_string();

	assert!(!recall_debug::note_debug_read_allowed(
		&note,
		"owner-agent",
		&allowed_scopes,
		&shared_grants,
		now
	));

	note.status = "active".to_string();
	note.expires_at = Some(now);

	assert!(!recall_debug::note_debug_read_allowed(
		&note,
		"owner-agent",
		&allowed_scopes,
		&shared_grants,
		now
	));

	note.expires_at = None;
	note.scope = "project_shared".to_string();

	assert!(!recall_debug::note_debug_read_allowed(
		&note,
		"other-agent",
		&allowed_scopes,
		&shared_grants,
		now
	));

	let shared_grants = HashSet::from([SharedSpaceGrantKey {
		scope: "project_shared".to_string(),
		space_owner_agent_id: "owner-agent".to_string(),
	}]);

	assert!(recall_debug::note_debug_read_allowed(
		&note,
		"other-agent",
		&allowed_scopes,
		&shared_grants,
		now
	));
}

#[test]
fn recall_trace_flattens_stale_and_dropped_context() {
	let layers = vec![
		recall_debug::layer_from_rows(
			"memory_notes",
			"pass",
			Some("trace".to_string()),
			"trace rows",
			vec![
				RecallDebugRow {
					layer: "memory_notes".to_string(),
					item_ref: serde_json::json!({"note_id": "selected-stale"}),
					selection_state: "selected".to_string(),
					authority_layer: "memory_note".to_string(),
					freshness_state: "deprecated".to_string(),
					source_refs: serde_json::json!([{"schema": "source_ref/v1"}]),
					score: Some(0.9),
					rank: Some(1),
					rationale: Some("selected but stale".to_string()),
					stage_reason: Some("status=deprecated".to_string()),
					replay_command: Some("elf_trace".to_string()),
					evidence_class: "pass".to_string(),
					debug_artifacts: serde_json::json!({}),
				},
				RecallDebugRow {
					layer: "memory_notes".to_string(),
					item_ref: serde_json::json!({"note_id": "dropped"}),
					selection_state: "dropped".to_string(),
					authority_layer: "memory_note".to_string(),
					freshness_state: "active".to_string(),
					source_refs: serde_json::json!([]),
					score: Some(0.4),
					rank: Some(4),
					rationale: Some("candidate not narrated".to_string()),
					stage_reason: Some("not_in_final_top_k".to_string()),
					replay_command: Some("elf_trace".to_string()),
					evidence_class: "pass".to_string(),
					debug_artifacts: serde_json::json!({}),
				},
			],
		),
		recall_debug::not_requested_layer("graph_facts", "missing graph subject"),
	];
	let trace = recall_debug::build_recall_trace(&layers);

	assert_eq!(trace.schema, "elf.recall_trace/v1");
	assert_eq!(trace.summary.entry_count, 3);
	assert_eq!(trace.summary.selected_count, 1);
	assert_eq!(trace.summary.dropped_count, 1);
	assert_eq!(trace.summary.stale_count, 1);
	assert_eq!(trace.summary.not_requested_count, 1);
	assert_eq!(trace.summary.replay_command_count, 2);
	assert_eq!(trace.entries[0].context_state, "stale");
	assert_eq!(trace.entries[0].policy_reason.as_deref(), Some("status=deprecated"));
	assert_eq!(trace.entries[1].context_state, "dropped");
	assert_eq!(trace.entries[1].policy_reason.as_deref(), Some("not_in_final_top_k"));
	assert_eq!(trace.entries[2].context_state, "not_requested");
}

#[test]
fn recall_trace_counts_blocked_layers_without_backend_details() {
	let layer = recall_debug::blocked_layer(
		"source_documents",
		Some("alpha".to_string()),
		"docs search failed",
		&Error::Storage { message: "password=secret host=db.internal".to_string() },
	);
	let trace = recall_debug::build_recall_trace(&[layer]);

	assert_eq!(trace.summary.blocked_count, 1);
	assert_eq!(trace.entries[0].context_state, "blocked");
	assert_eq!(trace.entries[0].selection_state, "blocked");
	assert!(
		trace.entries[0]
			.policy_reason
			.as_deref()
			.is_some_and(|reason| reason.contains("error_class=storage_unavailable"))
	);
	assert!(
		trace.entries[0]
			.policy_reason
			.as_deref()
			.is_some_and(|reason| !reason.contains("password=secret"))
	);
}

fn note_for_debug_visibility(agent_id: &str, scope: &str, status: &str) -> MemoryNote {
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
