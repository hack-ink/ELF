use super::*;

pub(super) fn build_trace_audit(actor_id: &str, token_id: Option<&str>) -> Value {
	match token_id.map(str::trim).filter(|value| !value.is_empty()) {
		Some(token_id) => serde_json::json!({ "actor_id": actor_id, "token_id": token_id }),
		None => serde_json::json!({ "actor_id": actor_id }),
	}
}

pub(super) fn build_trace_trajectory_stages(
	args: &BuildTraceArgs<'_>,
) -> Vec<TraceTrajectoryStageRecord> {
	let path_label = raw_search_path_label(args.path);

	vec![
		build_trace_rewrite_stage(args, path_label),
		build_trace_recall_stage(args, path_label),
		build_trace_fusion_stage(args, path_label),
		build_trace_rerank_stage(args, path_label),
		build_trace_final_stage(args, path_label),
	]
}

pub(super) fn build_trace_rewrite_stage(
	args: &BuildTraceArgs<'_>,
	path_label: &str,
) -> TraceTrajectoryStageRecord {
	let expanded_queries = sorted_unique_strings(args.expanded_queries.clone());

	TraceTrajectoryStageRecord {
		stage_id: Uuid::new_v4(),
		stage_order: 1,
		stage_name: "rewrite.expansion".to_string(),
		stage_payload: serde_json::json!({
			"schema": SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1,
			"path": path_label,
			"inputs": {
				"query": args.query,
				"expansion_mode": ranking::expansion_mode_label(args.expansion_mode),
			},
			"outputs": {
				"expanded_queries": expanded_queries,
			},
			"stats": {
				"expanded_query_count": args.expanded_queries.len(),
			},
		}),
		created_at: args.now,
		items: Vec::new(),
	}
}

pub(super) fn build_trace_recall_stage(
	args: &BuildTraceArgs<'_>,
	path_label: &str,
) -> TraceTrajectoryStageRecord {
	let mut stage_payload = serde_json::json!({
		"schema": SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1,
		"path": path_label,
		"stats": {
			"candidate_count_before_filter": args.candidate_count,
			"candidate_count_after_filter": args.filtered_candidate_count,
			"snippet_count": args.snippet_count,
		},
	});

	if let Some(filter_impact) = &args.filter_impact
		&& let Some(payload) = stage_payload.as_object_mut()
	{
		payload.insert("filter_impact".to_string(), filter_impact.to_stage_payload());
	}
	if let Some(recursive_retrieval) = args.recursive_retrieval
		&& recursive_retrieval.enabled
		&& let Some(payload) = stage_payload.as_object_mut()
	{
		payload.insert(
			"recursive".to_string(),
			serde_json::json!({
				"enabled": true,
				"scopes_seeded": recursive_retrieval.scopes_seeded,
				"scopes_queried": recursive_retrieval.scopes_queried,
				"candidates_before": recursive_retrieval.candidates_before,
				"candidates_added": recursive_retrieval.candidates_added,
				"candidates_after": recursive_retrieval.candidates_after,
				"rounds_executed": recursive_retrieval.rounds_executed,
				"total_queries": recursive_retrieval.total_queries,
				"stop_reason": recursive_retrieval
					.stop_reason
					.clone()
					.unwrap_or_else(|| "converged".to_string()),
			}),
		);
	}

	let items: Vec<TraceTrajectoryStageItemRecord> = args
		.recall_candidates
		.iter()
		.take(MAX_TRAJECTORY_STAGE_ITEMS)
		.map(|candidate| TraceTrajectoryStageItemRecord {
			id: Uuid::new_v4(),
			item_id: None,
			note_id: Some(candidate.note_id),
			chunk_id: Some(candidate.chunk_id),
			metrics: serde_json::json!({
				"retrieval_rank": candidate.retrieval_rank,
				"chunk_index": candidate.chunk_index,
			}),
		})
		.collect();

	TraceTrajectoryStageRecord {
		stage_id: Uuid::new_v4(),
		stage_order: 2,
		stage_name: "recall.candidates".to_string(),
		stage_payload,
		created_at: args.now,
		items,
	}
}

pub(super) fn build_trace_fusion_stage(
	args: &BuildTraceArgs<'_>,
	path_label: &str,
) -> TraceTrajectoryStageRecord {
	let items: Vec<TraceTrajectoryStageItemRecord> = args
		.fused_results
		.iter()
		.take(MAX_TRAJECTORY_STAGE_ITEMS)
		.map(|scored| TraceTrajectoryStageItemRecord {
			id: Uuid::new_v4(),
			item_id: None,
			note_id: Some(scored.item.note.note_id),
			chunk_id: Some(scored.item.chunk.chunk_id),
			metrics: serde_json::json!({
				"retrieval_rank": scored.item.retrieval_rank,
				"final_score": scored.final_score,
			}),
		})
		.collect();

	TraceTrajectoryStageRecord {
		stage_id: Uuid::new_v4(),
		stage_order: 3,
		stage_name: "fusion.merge".to_string(),
		stage_payload: serde_json::json!({
			"schema": SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1,
			"path": path_label,
			"stats": {
				"scored_count": args.scored_count,
				"fused_count": args.fused_count,
			},
			"decisions": {
				"fusion_weight": args.policies.retrieval_sources_policy.fusion_weight,
				"structured_field_weight": args.policies.retrieval_sources_policy.structured_field_weight,
				"fusion_priority": args.policies.retrieval_sources_policy.fusion_priority,
				"structured_field_priority": args.policies.retrieval_sources_policy.structured_field_priority,
			},
		}),
		created_at: args.now,
		items,
	}
}

pub(super) fn build_trace_rerank_stage(
	args: &BuildTraceArgs<'_>,
	path_label: &str,
) -> TraceTrajectoryStageRecord {
	let items: Vec<TraceTrajectoryStageItemRecord> = args
		.fused_results
		.iter()
		.take(MAX_TRAJECTORY_STAGE_ITEMS)
		.map(|scored| TraceTrajectoryStageItemRecord {
			id: Uuid::new_v4(),
			item_id: None,
			note_id: Some(scored.item.note.note_id),
			chunk_id: Some(scored.item.chunk.chunk_id),
			metrics: serde_json::json!({
				"rerank_score": scored.rerank_score,
				"rerank_rank": scored.rerank_rank,
				"rerank_norm": scored.rerank_norm,
				"retrieval_norm": scored.retrieval_norm,
				"final_score": scored.final_score,
			}),
		})
		.collect();

	TraceTrajectoryStageRecord {
		stage_id: Uuid::new_v4(),
		stage_order: 4,
		stage_name: "rerank.score".to_string(),
		stage_payload: serde_json::json!({
			"schema": SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1,
			"path": path_label,
			"stats": {
				"reranked_count": args.scored_count,
			},
			"decisions": {
				"blend_enabled": args.policies.blend_policy.enabled,
				"diversity_enabled": args.policies.diversity_policy.enabled,
			},
		}),
		created_at: args.now,
		items,
	}
}

pub(super) fn build_trace_final_stage(
	args: &BuildTraceArgs<'_>,
	path_label: &str,
) -> TraceTrajectoryStageRecord {
	TraceTrajectoryStageRecord {
		stage_id: Uuid::new_v4(),
		stage_order: 5,
		stage_name: "selection.final".to_string(),
		stage_payload: serde_json::json!({
			"schema": SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1,
			"path": path_label,
			"stats": {
				"selected_count": args.selected_count,
				"top_k": args.top_k,
			},
		}),
		created_at: args.now,
		items: Vec::new(),
	}
}
