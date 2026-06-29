use super::*;

pub(super) fn score_replay_candidate(
	ctx: &ScoreCandidateCtx<'_, '_>,
	candidate: &TraceReplayCandidate,
	rerank_rank: u32,
) -> ScoredReplay {
	let importance = candidate.note_importance;
	let retrieval_rank = candidate.retrieval_rank;
	let age_days = (ctx.now - candidate.note_updated_at).as_seconds_f32() / 86_400.0;
	let decay = if ctx.cfg.ranking.recency_tau_days > 0.0 {
		(-age_days / ctx.cfg.ranking.recency_tau_days).exp()
	} else {
		1.0
	};
	let base = (1.0 + 0.6 * importance) * decay;
	let tie_breaker_score = ctx.cfg.ranking.tie_breaker_weight * base;
	let scope_context_boost =
		ctx.scope_context_boost_by_scope.get(candidate.note_scope.as_str()).copied().unwrap_or(0.0);
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
		candidate.snippet.as_str(),
		candidate.note_hit_count,
		candidate.note_last_hit_at,
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

	ScoredReplay {
		note_id: candidate.note_id,
		chunk_id: candidate.chunk_id,
		retrieval_rank,
		final_score,
		rerank_score: candidate.rerank_score,
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
		note_scope: candidate.note_scope.clone(),
		deterministic_lexical_overlap_ratio: det_terms.lexical_overlap_ratio,
		deterministic_lexical_bonus: det_terms.lexical_bonus,
		deterministic_hit_count: det_terms.hit_count,
		deterministic_last_hit_age_days: det_terms.last_hit_age_days,
		deterministic_hit_boost: det_terms.hit_boost,
		deterministic_decay_penalty: det_terms.decay_penalty,
	}
}

pub(super) fn should_replace_replay_best(existing: &ScoredReplay, scored: &ScoredReplay) -> bool {
	let ord = ranking::cmp_f32_desc(scored.final_score, existing.final_score);

	if ord != Ordering::Equal {
		ord == Ordering::Less
	} else {
		scored.retrieval_rank < existing.retrieval_rank
	}
}

pub(super) fn cmp_scored_replay(a: &ScoredReplay, b: &ScoredReplay) -> Ordering {
	let ord = ranking::cmp_f32_desc(a.final_score, b.final_score);

	if ord != Ordering::Equal {
		return ord;
	}

	let ord = a.retrieval_rank.cmp(&b.retrieval_rank);

	if ord != Ordering::Equal {
		return ord;
	}

	let ord = a.note_id.cmp(&b.note_id);

	if ord != Ordering::Equal {
		return ord;
	}

	a.chunk_id.cmp(&b.chunk_id)
}

pub(super) fn apply_replay_diversity_selection(
	mut results: Vec<ScoredReplay>,
	top_k: u32,
	diversity_enabled: bool,
	replay_diversity_decisions: &HashMap<Uuid, DiversityDecision>,
) -> Vec<ScoredReplay> {
	if diversity_enabled && !replay_diversity_decisions.is_empty() {
		let mut selected: Vec<ScoredReplay> = results
			.iter()
			.filter(|scored| {
				replay_diversity_decisions
					.get(&scored.note_id)
					.map(|decision| decision.selected)
					.unwrap_or(false)
			})
			.cloned()
			.collect();

		selected.sort_by(|a, b| {
			let rank_a = replay_diversity_decisions
				.get(&a.note_id)
				.and_then(|decision| decision.selected_rank)
				.unwrap_or(u32::MAX);
			let rank_b = replay_diversity_decisions
				.get(&b.note_id)
				.and_then(|decision| decision.selected_rank)
				.unwrap_or(u32::MAX);
			let ord = rank_a.cmp(&rank_b);

			if ord != Ordering::Equal {
				return ord;
			}

			a.note_id.cmp(&b.note_id)
		});

		if !selected.is_empty() {
			results = selected;
		}
	}

	results.truncate(top_k.max(1) as usize);

	results
}

pub(super) fn build_replay_items(
	cfg: &Config,
	blend_policy: &ResolvedBlendPolicy,
	diversity_policy: &ResolvedDiversityPolicy,
	policy_id: &str,
	replay_diversity_decisions: &HashMap<Uuid, DiversityDecision>,
	results: Vec<ScoredReplay>,
) -> Vec<TraceReplayItem> {
	let mut out = Vec::with_capacity(results.len());

	for scored in results {
		let terms = ranking_explain_v2::build_trace_terms_v2(TraceTermsArgs {
			cfg,
			blend_enabled: blend_policy.enabled,
			retrieval_normalization: blend_policy.retrieval_normalization.as_str(),
			rerank_normalization: blend_policy.rerank_normalization.as_str(),
			blend_retrieval_weight: scored.blend_retrieval_weight,
			retrieval_rank: scored.retrieval_rank,
			retrieval_norm: scored.retrieval_norm,
			retrieval_term: scored.retrieval_term,
			rerank_score: scored.rerank_score,
			rerank_rank: scored.rerank_rank,
			rerank_norm: scored.rerank_norm,
			rerank_term: scored.rerank_term,
			tie_breaker_score: scored.tie_breaker_score,
			importance: scored.importance,
			age_days: scored.age_days,
			scope: scored.note_scope.as_str(),
			scope_context_boost: scored.scope_context_boost,
			deterministic_lexical_overlap_ratio: scored.deterministic_lexical_overlap_ratio,
			deterministic_lexical_bonus: scored.deterministic_lexical_bonus,
			deterministic_hit_count: scored.deterministic_hit_count,
			deterministic_last_hit_age_days: scored.deterministic_last_hit_age_days,
			deterministic_hit_boost: scored.deterministic_hit_boost,
			deterministic_decay_penalty: scored.deterministic_decay_penalty,
		});
		let explain = SearchExplain {
			r#match: SearchMatchExplain { matched_terms: Vec::new(), matched_fields: Vec::new() },
			ranking: SearchRankingExplain {
				schema: SEARCH_RANKING_EXPLAIN_SCHEMA_V2.to_string(),
				policy_id: policy_id.to_string(),
				final_score: scored.final_score,
				terms,
			},
			relation_context: None,
			diversity: if diversity_policy.enabled {
				replay_diversity_decisions
					.get(&scored.note_id)
					.map(ranking::build_diversity_explain)
			} else {
				None
			},
		};

		out.push(TraceReplayItem {
			note_id: scored.note_id,
			chunk_id: scored.chunk_id,
			retrieval_rank: scored.retrieval_rank,
			final_score: scored.final_score,
			explain,
		});
	}

	out
}
