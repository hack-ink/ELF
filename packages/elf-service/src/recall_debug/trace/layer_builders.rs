use crate::recall_debug::{self, Error, RecallDebugLayer, RecallDebugRow, Value};

pub(in crate::recall_debug) fn layer_from_rows(
	layer: &str,
	evidence_class: &str,
	anchor: Option<String>,
	summary: &str,
	rows: Vec<RecallDebugRow>,
) -> RecallDebugLayer {
	layer_from_rows_with_artifacts(
		layer,
		evidence_class,
		anchor,
		summary,
		rows,
		serde_json::json!({}),
	)
}

pub(in crate::recall_debug) fn layer_from_rows_with_artifacts(
	layer: &str,
	evidence_class: &str,
	anchor: Option<String>,
	summary: &str,
	rows: Vec<RecallDebugRow>,
	debug_artifacts: Value,
) -> RecallDebugLayer {
	let selected_count = rows.iter().filter(|row| row.selection_state == "selected").count();
	let dropped_count = rows.iter().filter(|row| row.selection_state == "dropped").count();
	let available_count = rows
		.iter()
		.filter(|row| matches!(row.selection_state.as_str(), "available" | "reviewable"))
		.count();
	let replayable = rows.iter().any(|row| row.replay_command.is_some());

	RecallDebugLayer {
		layer: layer.to_string(),
		evidence_class: evidence_class.to_string(),
		summary: summary.to_string(),
		anchor,
		row_count: rows.len(),
		selected_count,
		dropped_count,
		available_count,
		raw_sql_needed: false,
		replayable,
		debug_artifacts,
		rows,
	}
}

pub(in crate::recall_debug) fn not_requested_layer(layer: &str, summary: &str) -> RecallDebugLayer {
	RecallDebugLayer {
		layer: layer.to_string(),
		evidence_class: "not_requested".to_string(),
		summary: summary.to_string(),
		anchor: None,
		row_count: 0,
		selected_count: 0,
		dropped_count: 0,
		available_count: 0,
		raw_sql_needed: false,
		replayable: false,
		debug_artifacts: serde_json::json!({}),
		rows: Vec::new(),
	}
}

pub(in crate::recall_debug) fn blocked_layer(
	layer: &str,
	anchor: Option<String>,
	summary: &str,
	err: &Error,
) -> RecallDebugLayer {
	RecallDebugLayer {
		layer: layer.to_string(),
		evidence_class: "blocked".to_string(),
		summary: format!("{summary} error_class={}", recall_debug::public_error_class(err)),
		anchor,
		row_count: 0,
		selected_count: 0,
		dropped_count: 0,
		available_count: 0,
		raw_sql_needed: false,
		replayable: false,
		debug_artifacts: serde_json::json!({}),
		rows: Vec::new(),
	}
}
