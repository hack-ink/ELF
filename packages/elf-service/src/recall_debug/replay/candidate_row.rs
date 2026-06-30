use crate::recall_debug::{self, NoteDebugSourceRow, RecallDebugRow, TraceReplayCandidate, Uuid};

pub(in crate::recall_debug) fn candidate_debug_row(
	trace_id: Uuid,
	candidate: &TraceReplayCandidate,
	source: Option<&NoteDebugSourceRow>,
	replay_command: &str,
) -> RecallDebugRow {
	let selected_by_diversity = candidate.diversity_selected.unwrap_or(false);
	let skipped_reason = candidate.diversity_skipped_reason.clone().or_else(|| {
		if selected_by_diversity {
			candidate.diversity_selected_reason.clone()
		} else {
			Some("not_in_final_top_k".to_string())
		}
	});

	RecallDebugRow {
		layer: "memory_notes".to_string(),
		item_ref: serde_json::json!({
			"trace_id": trace_id,
			"note_id": candidate.note_id,
			"chunk_id": candidate.chunk_id,
			"chunk_index": candidate.chunk_index,
		}),
		selection_state: "dropped".to_string(),
		authority_layer: "memory_note".to_string(),
		freshness_state: recall_debug::freshness_from_note_source(source),
		source_refs: recall_debug::source_ref_from_note_source(source),
		score: candidate.retrieval_score,
		rank: Some(candidate.retrieval_rank),
		rationale: Some(
			"candidate captured for replay but not selected in final result set".to_string(),
		),
		stage_reason: skipped_reason,
		replay_command: Some(replay_command.to_string()),
		evidence_class: "pass".to_string(),
		debug_artifacts: serde_json::json!({
			"snippet": candidate.snippet,
			"rerank_score": candidate.rerank_score,
			"note_scope": candidate.note_scope,
			"diversity_selected": candidate.diversity_selected,
			"diversity_selected_rank": candidate.diversity_selected_rank,
			"diversity_nearest_selected_note_id": candidate.diversity_nearest_selected_note_id,
			"diversity_similarity": candidate.diversity_similarity,
			"diversity_mmr_score": candidate.diversity_mmr_score,
			"diversity_missing_embedding": candidate.diversity_missing_embedding,
		}),
	}
}
