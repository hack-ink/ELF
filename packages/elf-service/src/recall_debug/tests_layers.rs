use crate::{
	RecallDebugRow,
	recall_debug::{self, BTreeSet, Error, Uuid},
};

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
