use crate::recall_debug::{
	RecallDebugLayer, RecallDebugPanelSummary, RecallTraceEntry, RecallTraceSummary,
};

pub(in crate::recall_debug) fn summarize_layers(
	layers: &[RecallDebugLayer],
) -> RecallDebugPanelSummary {
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

pub(in crate::recall_debug::trace) fn summarize_trace_entries(
	entries: &[RecallTraceEntry],
) -> RecallTraceSummary {
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

pub(in crate::recall_debug::trace) fn stale_freshness_state(freshness_state: &str) -> bool {
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
