use crate::recall_debug::{
	ELF_RECALL_TRACE_SCHEMA_V1, RecallDebugLayer, RecallDebugRow, RecallTrace, RecallTraceEntry,
	trace::summary,
};

pub(in crate::recall_debug) fn build_recall_trace(layers: &[RecallDebugLayer]) -> RecallTrace {
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

	let summary = summary::summarize_trace_entries(&entries);

	RecallTrace { schema: ELF_RECALL_TRACE_SCHEMA_V1.to_string(), summary, entries }
}

fn layer_trace_entry(layer: &RecallDebugLayer) -> RecallTraceEntry {
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

fn row_trace_entry(row: &RecallDebugRow) -> RecallTraceEntry {
	let context_state = if summary::stale_freshness_state(&row.freshness_state) {
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
