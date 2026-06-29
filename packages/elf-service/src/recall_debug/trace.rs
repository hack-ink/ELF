use super::*;

pub(super) fn summarize_layers(layers: &[RecallDebugLayer]) -> RecallDebugPanelSummary {
	let mut summary = RecallDebugPanelSummary { layer_count: layers.len(), ..Default::default() };

	for layer in layers {
		summary.row_count += layer.row_count;
		summary.selected_count += layer.selected_count;
		summary.dropped_count += layer.dropped_count;
		summary.available_count += layer.available_count;

		if layer.evidence_class == "not_requested" {
			summary.not_requested_layer_count += 1;
		}
		if matches!(layer.evidence_class.as_str(), "incomplete" | "blocked" | "wrong_result") {
			summary.incomplete_layer_count += 1;
		}
		if layer.raw_sql_needed {
			summary.raw_sql_needed_count += 1;
		}

		summary.replay_command_count += layer
			.rows
			.iter()
			.filter(|row| row.replay_command.as_ref().is_some_and(|value| !value.is_empty()))
			.count();
		*summary.evidence_class_counts.entry(layer.evidence_class.clone()).or_default() += 1;
	}

	summary
}

pub(super) fn build_recall_trace(layers: &[RecallDebugLayer]) -> RecallTrace {
	let mut entries = Vec::new();

	for layer in layers {
		if layer.rows.is_empty() {
			if matches!(
				layer.evidence_class.as_str(),
				"blocked" | "not_requested" | "incomplete" | "wrong_result"
			) {
				entries.push(layer_trace_entry(layer));
			}

			continue;
		}

		entries.extend(layer.rows.iter().map(row_trace_entry));
	}

	let summary = summarize_trace_entries(&entries);

	RecallTrace { schema: ELF_RECALL_TRACE_SCHEMA_V1.to_string(), summary, entries }
}

pub(super) fn summarize_trace_entries(entries: &[RecallTraceEntry]) -> RecallTraceSummary {
	let mut summary = RecallTraceSummary { entry_count: entries.len(), ..Default::default() };

	for entry in entries {
		match entry.selection_state.as_str() {
			"selected" => summary.selected_count += 1,
			"dropped" => summary.dropped_count += 1,
			"blocked" => summary.blocked_count += 1,
			"not_requested" => summary.not_requested_count += 1,
			_ => {},
		}

		if entry.context_state == "stale" || stale_freshness_state(&entry.freshness_state) {
			summary.stale_count += 1;
		}
		if entry.raw_sql_needed {
			summary.raw_sql_needed_count += 1;
		}
		if entry.replay_command.as_ref().is_some_and(|value| !value.is_empty()) {
			summary.replay_command_count += 1;
		}
	}

	summary
}

pub(super) fn layer_trace_entry(layer: &RecallDebugLayer) -> RecallTraceEntry {
	let context_state = match layer.evidence_class.as_str() {
		"not_requested" => "not_requested",
		"blocked" => "blocked",
		"incomplete" => "incomplete",
		"wrong_result" => "wrong_result",
		_ => "available",
	};

	RecallTraceEntry {
		layer: layer.layer.clone(),
		context_state: context_state.to_string(),
		selection_state: layer.evidence_class.clone(),
		authority_layer: layer.layer.clone(),
		freshness_state: layer.evidence_class.clone(),
		item_ref: serde_json::json!({
			"layer": layer.layer.clone(),
			"anchor": layer.anchor.clone(),
		}),
		source_refs: serde_json::json!([]),
		score: None,
		rank: None,
		policy_reason: Some(layer.summary.clone()),
		replay_command: None,
		evidence_class: layer.evidence_class.clone(),
		raw_sql_needed: layer.raw_sql_needed,
	}
}

pub(super) fn row_trace_entry(row: &RecallDebugRow) -> RecallTraceEntry {
	let context_state = if stale_freshness_state(&row.freshness_state) {
		"stale"
	} else {
		row.selection_state.as_str()
	};

	RecallTraceEntry {
		layer: row.layer.clone(),
		context_state: context_state.to_string(),
		selection_state: row.selection_state.clone(),
		authority_layer: row.authority_layer.clone(),
		freshness_state: row.freshness_state.clone(),
		item_ref: row.item_ref.clone(),
		source_refs: row.source_refs.clone(),
		score: row.score,
		rank: row.rank,
		policy_reason: row.stage_reason.clone().or_else(|| row.rationale.clone()),
		replay_command: row.replay_command.clone(),
		evidence_class: row.evidence_class.clone(),
		raw_sql_needed: false,
	}
}

pub(super) fn stale_freshness_state(freshness_state: &str) -> bool {
	matches!(
		freshness_state,
		"stale"
			| "deprecated"
			| "deleted"
			| "superseded"
			| "tombstoned"
			| "historical"
			| "archived"
			| "lint_warning"
			| "lint_error"
	)
}

pub(super) fn layer_from_rows(
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

pub(super) fn layer_from_rows_with_artifacts(
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

pub(super) fn not_requested_layer(layer: &str, summary: &str) -> RecallDebugLayer {
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

pub(super) fn blocked_layer(
	layer: &str,
	anchor: Option<String>,
	summary: &str,
	err: &Error,
) -> RecallDebugLayer {
	RecallDebugLayer {
		layer: layer.to_string(),
		evidence_class: "blocked".to_string(),
		summary: format!("{summary} error_class={}", public_error_class(err)),
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
