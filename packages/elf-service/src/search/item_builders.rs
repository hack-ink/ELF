use crate::{
	ranking_explain_v2,
	search::{
		BuildSearchItemArgs, MAX_MATCHED_TERMS, OffsetDateTime, SEARCH_RANKING_EXPLAIN_SCHEMA_V2,
		ScoredChunk, SearchExplain, SearchItem, SearchMatchExplain, SearchRankingExplain,
		TraceCandidateRecord, TraceItemRecord, TraceReplayCandidate, TraceTermsArgs, Uuid, ranking,
	},
};

pub(super) fn build_trace_candidate_record(
	scored_chunk: &ScoredChunk,
	now: OffsetDateTime,
	expires_at: OffsetDateTime,
) -> TraceCandidateRecord {
	let note = &scored_chunk.item.note;

	TraceCandidateRecord {
		candidate_id: Uuid::new_v4(),
		note_id: note.note_id,
		chunk_id: scored_chunk.item.chunk.chunk_id,
		chunk_index: scored_chunk.item.chunk.chunk_index,
		snippet: scored_chunk.item.snippet.clone(),
		candidate_snapshot: serde_json::to_value(TraceReplayCandidate {
			note_id: note.note_id,
			chunk_id: scored_chunk.item.chunk.chunk_id,
			chunk_index: scored_chunk.item.chunk.chunk_index,
			snippet: scored_chunk.item.snippet.clone(),
			retrieval_rank: scored_chunk.item.retrieval_rank,
			retrieval_score: scored_chunk.item.retrieval_score,
			rerank_score: scored_chunk.rerank_score,
			note_scope: note.scope.clone(),
			note_importance: note.importance,
			note_updated_at: note.updated_at,
			note_hit_count: note.hit_count,
			note_last_hit_at: note.last_hit_at,
			diversity_selected: None,
			diversity_selected_rank: None,
			diversity_selected_reason: None,
			diversity_skipped_reason: None,
			diversity_nearest_selected_note_id: None,
			diversity_similarity: None,
			diversity_mmr_score: None,
			diversity_missing_embedding: None,
		})
		.unwrap_or_else(|_| serde_json::json!({})),
		retrieval_rank: scored_chunk.item.retrieval_rank,
		rerank_score: scored_chunk.rerank_score,
		note_scope: note.scope.clone(),
		note_importance: note.importance,
		note_updated_at: note.updated_at,
		note_hit_count: note.hit_count,
		note_last_hit_at: note.last_hit_at,
		created_at: now,
		expires_at,
	}
}

pub(super) fn build_search_item_and_trace_item(
	args: BuildSearchItemArgs<'_>,
) -> (SearchItem, TraceItemRecord) {
	let (matched_terms, matched_fields) = ranking::match_terms_in_text(
		args.query_tokens,
		args.scored_chunk.item.snippet.as_str(),
		args.scored_chunk.item.note.key.as_deref(),
		MAX_MATCHED_TERMS,
	);
	let matched_fields = ranking::merge_matched_fields(
		matched_fields,
		args.structured_matches.get(&args.scored_chunk.item.note.note_id),
	);
	let trace_terms = ranking_explain_v2::build_trace_terms_v2(TraceTermsArgs {
		cfg: args.cfg,
		blend_enabled: args.blend_policy.enabled,
		retrieval_normalization: args.blend_policy.retrieval_normalization.as_str(),
		rerank_normalization: args.blend_policy.rerank_normalization.as_str(),
		blend_retrieval_weight: args.scored_chunk.blend_retrieval_weight,
		retrieval_rank: args.scored_chunk.item.retrieval_rank,
		retrieval_norm: args.scored_chunk.retrieval_norm,
		retrieval_term: args.scored_chunk.retrieval_term,
		rerank_score: args.scored_chunk.rerank_score,
		rerank_rank: args.scored_chunk.rerank_rank,
		rerank_norm: args.scored_chunk.rerank_norm,
		rerank_term: args.scored_chunk.rerank_term,
		tie_breaker_score: args.scored_chunk.tie_breaker_score,
		importance: args.scored_chunk.importance,
		age_days: args.scored_chunk.age_days,
		scope: args.scored_chunk.item.note.scope.as_str(),
		scope_context_boost: args.scored_chunk.scope_context_boost,
		deterministic_lexical_overlap_ratio: args.scored_chunk.deterministic_lexical_overlap_ratio,
		deterministic_lexical_bonus: args.scored_chunk.deterministic_lexical_bonus,
		deterministic_hit_count: args.scored_chunk.deterministic_hit_count,
		deterministic_last_hit_age_days: args.scored_chunk.deterministic_last_hit_age_days,
		deterministic_hit_boost: args.scored_chunk.deterministic_hit_boost,
		deterministic_decay_penalty: args.scored_chunk.deterministic_decay_penalty,
	});
	let response_terms = ranking_explain_v2::strip_term_inputs(&trace_terms);
	let relation_context =
		args.relation_contexts.get(&args.scored_chunk.item.note.note_id).cloned();
	let diversity = if args.diversity_policy.enabled {
		args.diversity_decisions
			.get(&args.scored_chunk.item.note.note_id)
			.map(ranking::build_diversity_explain)
	} else {
		None
	};
	let response_explain = SearchExplain {
		r#match: SearchMatchExplain {
			matched_terms: matched_terms.clone(),
			matched_fields: matched_fields.clone(),
		},
		ranking: SearchRankingExplain {
			schema: SEARCH_RANKING_EXPLAIN_SCHEMA_V2.to_string(),
			policy_id: args.policy_id.to_string(),
			final_score: args.scored_chunk.final_score,
			terms: response_terms,
		},
		relation_context: relation_context.clone(),
		diversity: diversity.clone(),
	};
	let trace_explain = SearchExplain {
		r#match: SearchMatchExplain { matched_terms, matched_fields },
		ranking: SearchRankingExplain {
			schema: SEARCH_RANKING_EXPLAIN_SCHEMA_V2.to_string(),
			policy_id: args.policy_id.to_string(),
			final_score: args.scored_chunk.final_score,
			terms: trace_terms,
		},
		relation_context,
		diversity,
	};
	let result_handle = Uuid::new_v4();
	let note = &args.scored_chunk.item.note;
	let chunk = &args.scored_chunk.item.chunk;
	let item = SearchItem {
		result_handle,
		note_id: note.note_id,
		chunk_id: chunk.chunk_id,
		chunk_index: chunk.chunk_index,
		start_offset: chunk.start_offset,
		end_offset: chunk.end_offset,
		snippet: args.scored_chunk.item.snippet.clone(),
		r#type: note.note_type.clone(),
		key: note.key.clone(),
		scope: note.scope.clone(),
		importance: note.importance,
		confidence: note.confidence,
		updated_at: note.updated_at,
		expires_at: note.expires_at,
		final_score: args.scored_chunk.final_score,
		source_ref: note.source_ref.clone(),
		explain: response_explain,
	};
	let trace_item = TraceItemRecord {
		item_id: result_handle,
		note_id: note.note_id,
		chunk_id: Some(chunk.chunk_id),
		rank: args.rank,
		final_score: args.scored_chunk.final_score,
		explain: trace_explain,
	};

	(item, trace_item)
}
