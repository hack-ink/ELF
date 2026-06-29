use super::*;

pub(super) fn select_best_scored_chunks(scored: Vec<ScoredChunk>) -> Vec<ScoredChunk> {
	let mut best_by_note: HashMap<Uuid, ScoredChunk> = HashMap::new();

	for scored_item in scored {
		let note_id = scored_item.item.note.note_id;
		let replace = match best_by_note.get(&note_id) {
			Some(existing) => scored_item.final_score > existing.final_score,
			None => true,
		};

		if replace {
			best_by_note.insert(note_id, scored_item);
		}
	}

	let mut results: Vec<ScoredChunk> = best_by_note.into_values().collect();

	results.sort_by(cmp_scored_chunk);

	results
}

pub(super) fn cmp_scored_chunk(a: &ScoredChunk, b: &ScoredChunk) -> Ordering {
	let ord = ranking::cmp_f32_desc(a.final_score, b.final_score);

	if ord != Ordering::Equal {
		return ord;
	}

	let ord = a.item.retrieval_rank.cmp(&b.item.retrieval_rank);

	if ord != Ordering::Equal {
		return ord;
	}

	let ord = a.item.note.note_id.cmp(&b.item.note.note_id);

	if ord != Ordering::Equal {
		return ord;
	}

	a.item.chunk.chunk_id.cmp(&b.item.chunk.chunk_id)
}

pub(super) fn score_chunk_candidate(
	ctx: &ScoreCandidateCtx<'_, '_>,
	item: ChunkSnippet,
	rerank_score: f32,
	rerank_rank: u32,
) -> ScoredChunk {
	let importance = item.note.importance;
	let retrieval_rank = item.retrieval_rank;
	let age_days = (ctx.now - item.note.updated_at).as_seconds_f32() / 86_400.0;
	let decay = if ctx.cfg.ranking.recency_tau_days > 0.0 {
		(-age_days / ctx.cfg.ranking.recency_tau_days).exp()
	} else {
		1.0
	};
	let base = (1.0 + 0.6 * importance) * decay;
	let tie_breaker_score = ctx.cfg.ranking.tie_breaker_weight * base;
	let scope_context_boost =
		ctx.scope_context_boost_by_scope.get(item.note.scope.as_str()).copied().unwrap_or(0.0);
	let rerank_norm = match ctx.blend_policy.rerank_normalization {
		NormalizationKind::Rank => ranking::rank_normalize(rerank_rank, ctx.total_rerank),
	};
	let retrieval_norm = match ctx.blend_policy.retrieval_normalization {
		NormalizationKind::Rank => ranking::rank_normalize(retrieval_rank, ctx.total_retrieval),
	};
	let blend_retrieval_weight = if ctx.blend_policy.enabled {
		ranking::retrieval_weight_for_rank(retrieval_rank, &ctx.blend_policy.segments)
	} else {
		0.0
	};
	let retrieval_term = blend_retrieval_weight * retrieval_norm;
	let rerank_term = (1.0 - blend_retrieval_weight) * rerank_norm;
	let det_terms = ranking::compute_deterministic_ranking_terms(
		ctx.cfg,
		ctx.det_query_tokens,
		item.snippet.as_str(),
		item.note.hit_count,
		item.note.last_hit_at,
		age_days,
		ctx.now,
	);
	let final_score = retrieval_term
		+ rerank_term
		+ tie_breaker_score
		+ scope_context_boost
		+ det_terms.lexical_bonus
		+ det_terms.hit_boost
		+ det_terms.decay_penalty;

	ScoredChunk {
		item,
		final_score,
		rerank_score,
		rerank_rank,
		rerank_norm,
		retrieval_norm,
		blend_retrieval_weight,
		retrieval_term,
		rerank_term,
		tie_breaker_score,
		scope_context_boost,
		age_days,
		importance,
		deterministic_lexical_overlap_ratio: det_terms.lexical_overlap_ratio,
		deterministic_lexical_bonus: det_terms.lexical_bonus,
		deterministic_hit_count: det_terms.hit_count,
		deterministic_last_hit_age_days: det_terms.last_hit_age_days,
		deterministic_hit_boost: det_terms.hit_boost,
		deterministic_decay_penalty: det_terms.decay_penalty,
	}
}
