use crate::recall_debug::{
	self, BTreeMap, BTreeSet, NoteDebugSourceRow, TraceReplayCandidate, Uuid, Value,
};

pub(in crate::recall_debug::replay) fn compact_candidate_replay(
	candidates: &[TraceReplayCandidate],
	selected_candidate_keys: &BTreeSet<(Uuid, Uuid)>,
	source_refs: &BTreeMap<Uuid, NoteDebugSourceRow>,
) -> Value {
	let rerank_ranks = candidate_rerank_ranks(candidates);
	let rows = candidates
		.iter()
		.map(|candidate| {
			let key = recall_debug::candidate_identity(candidate.note_id, candidate.chunk_id);
			let rerank_rank = rerank_ranks.get(&key).copied();
			let selection_state =
				if selected_candidate_keys.contains(&key) { "selected" } else { "dropped" };
			let stage_reason = candidate_stage_reason(candidate, selection_state);
			let source_ref =
				source_refs.get(&candidate.note_id).map(|source| source.source_ref.clone());

			serde_json::json!({
				"note_id": candidate.note_id,
				"chunk_id": candidate.chunk_id,
				"source_ref": source_ref,
				"source_ref_available": source_ref.is_some(),
				"retrieval_rank": candidate.retrieval_rank,
				"rerank_rank": rerank_rank,
				"rerank_delta": rerank_rank.map(|rank| candidate.retrieval_rank as i64 - i64::from(rank)),
				"rerank_score": candidate.rerank_score,
				"retrieval_score": candidate.retrieval_score,
				"selection_state": selection_state,
				"stage_reason": stage_reason,
				"policy_reason": stage_reason,
				"note_scope": candidate.note_scope,
				"diversity_selected": candidate.diversity_selected,
				"diversity_skipped_reason": candidate.diversity_skipped_reason,
			})
		})
		.collect::<Vec<_>>();
	let selected_count = rows
		.iter()
		.filter(|row| row.get("selection_state").and_then(Value::as_str) == Some("selected"))
		.count();

	serde_json::json!({
		"candidate_count": candidates.len(),
		"selected_count": selected_count,
		"dropped_count": rows.len().saturating_sub(selected_count),
		"rows": rows,
	})
}

fn candidate_rerank_ranks(candidates: &[TraceReplayCandidate]) -> BTreeMap<(Uuid, Uuid), u32> {
	let mut ordered = candidates.iter().collect::<Vec<_>>();

	ordered.sort_by(|a, b| {
		b.rerank_score
			.total_cmp(&a.rerank_score)
			.then_with(|| a.retrieval_rank.cmp(&b.retrieval_rank))
			.then_with(|| a.note_id.cmp(&b.note_id))
			.then_with(|| a.chunk_id.cmp(&b.chunk_id))
	});

	ordered
		.into_iter()
		.enumerate()
		.map(|(index, candidate)| {
			(
				recall_debug::candidate_identity(candidate.note_id, candidate.chunk_id),
				index as u32 + 1,
			)
		})
		.collect()
}

fn candidate_stage_reason(candidate: &TraceReplayCandidate, selection_state: &str) -> String {
	if selection_state == "selected" {
		candidate.diversity_selected_reason.clone().unwrap_or_else(|| "selection.final".to_string())
	} else {
		candidate
			.diversity_skipped_reason
			.clone()
			.unwrap_or_else(|| "not_in_final_top_k".to_string())
	}
}
