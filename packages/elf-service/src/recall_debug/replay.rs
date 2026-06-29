use crate::recall_debug::{
	self, BTreeMap, BTreeSet, NoteDebugSourceRow, RecallDebugRow, SearchExplainItem, SearchTrace,
	SearchTrajectoryStage, TraceReplayCandidate, Uuid, Value,
};

pub(super) fn candidate_debug_row(
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

pub(super) fn memory_compact_replay_artifact(
	trace: &SearchTrace,
	stages: &[SearchTrajectoryStage],
	candidates: &[TraceReplayCandidate],
	selected_items: &[&SearchExplainItem],
	selected_candidate_keys: &BTreeSet<(Uuid, Uuid)>,
	source_refs: &BTreeMap<Uuid, NoteDebugSourceRow>,
	replay_command: &str,
) -> Value {
	serde_json::json!({
		"schema": "elf.recall_debug.compact_replay/v1",
		"trace_id": trace.trace_id,
		"query": trace.query,
		"replay_command": replay_command,
		"controls": compact_replay_controls(trace),
		"stage_movement": compact_stage_movement(stages),
		"candidate_replay": compact_candidate_replay(candidates, selected_candidate_keys, source_refs),
		"selected_context": compact_selected_context(selected_items, source_refs),
		"authority": {
			"source_refs_visible": true,
			"policy_reasons_visible": true,
			"raw_sql_needed": false,
		},
	})
}

pub(super) fn compact_replay_controls(trace: &SearchTrace) -> Value {
	serde_json::json!({
		"top_k": trace.top_k,
		"candidate_count": trace.candidate_count,
		"expansion_mode": trace.expansion_mode,
		"expanded_query_count": trace.expanded_queries.len(),
		"expanded_queries": trace.expanded_queries,
		"allowed_scopes": trace.allowed_scopes,
		"search": compact_pointer(&trace.config_snapshot, "/search"),
		"ranking": {
			"policy_id": compact_pointer(&trace.config_snapshot, "/ranking/policy_id"),
			"blend": compact_pointer(&trace.config_snapshot, "/ranking/blend"),
			"diversity": compact_pointer(&trace.config_snapshot, "/ranking/diversity"),
			"retrieval_sources": compact_pointer(&trace.config_snapshot, "/ranking/retrieval_sources"),
			"override": compact_pointer(&trace.config_snapshot, "/ranking/override"),
		},
	})
}

pub(super) fn compact_stage_movement(stages: &[SearchTrajectoryStage]) -> Vec<Value> {
	stages
		.iter()
		.map(|stage| {
			serde_json::json!({
				"stage_order": stage.stage_order,
				"stage_name": stage.stage_name,
				"item_count": stage.items.len(),
				"stats": compact_pointer(&stage.stage_payload, "/stats"),
				"decisions": compact_pointer(&stage.stage_payload, "/decisions"),
				"filter_impact": compact_pointer(&stage.stage_payload, "/filter_impact"),
			})
		})
		.collect()
}

pub(super) fn compact_candidate_replay(
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

pub(super) fn candidate_rerank_ranks(
	candidates: &[TraceReplayCandidate],
) -> BTreeMap<(Uuid, Uuid), u32> {
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

pub(super) fn candidate_stage_reason(
	candidate: &TraceReplayCandidate,
	selection_state: &str,
) -> String {
	if selection_state == "selected" {
		candidate.diversity_selected_reason.clone().unwrap_or_else(|| "selection.final".to_string())
	} else {
		candidate
			.diversity_skipped_reason
			.clone()
			.unwrap_or_else(|| "not_in_final_top_k".to_string())
	}
}

pub(super) fn compact_selected_context(
	selected_items: &[&SearchExplainItem],
	source_refs: &BTreeMap<Uuid, NoteDebugSourceRow>,
) -> Vec<Value> {
	selected_items
		.iter()
		.map(|item| {
			let source = source_refs.get(&item.note_id);

			serde_json::json!({
				"result_handle": item.result_handle,
				"note_id": item.note_id,
				"chunk_id": item.chunk_id,
				"source_ref": source.map(|row| row.source_ref.clone()),
				"source_ref_available": source.is_some(),
				"freshness_state": recall_debug::freshness_from_note_source(source),
				"final_rank": item.rank,
				"final_score": item.explain.ranking.final_score,
				"policy_id": item.explain.ranking.policy_id,
				"policy_reason": "final ranked search result",
				"ranking_terms": item
					.explain
					.ranking
					.terms
					.iter()
					.map(|term| serde_json::json!({
						"name": term.name,
						"value": term.value,
					}))
					.collect::<Vec<_>>(),
				"relation_context_count": item
					.explain
					.relation_context
					.as_ref()
					.map(Vec::len)
					.unwrap_or_default(),
			})
		})
		.collect()
}

pub(super) fn compact_pointer(value: &Value, pointer: &str) -> Value {
	value.pointer(pointer).cloned().unwrap_or(Value::Null)
}
