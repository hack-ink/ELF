use std::collections::HashSet;

use time::OffsetDateTime;

use crate::{
	RecallDebugRow,
	access::SharedSpaceGrantKey,
	recall_debug::{self, Error, tests::tests_helpers},
};

#[test]
fn debug_note_readability_requires_current_note_and_scope_access() {
	let allowed_scopes = vec!["agent_private".to_string(), "project_shared".to_string()];
	let shared_grants = HashSet::new();
	let now = OffsetDateTime::now_utc();
	let mut note =
		tests_helpers::note_for_debug_visibility("owner-agent", "agent_private", "active");

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
