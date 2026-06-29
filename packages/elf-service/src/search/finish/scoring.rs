use crate::search::{
	self, ChunkCandidate, ChunkSnippet, DiversityDecision, ElfService, FinishSearchPolicies,
	FinishSearchScoringResult, HashMap, MAX_MATCHED_TERMS, NoteMeta, OffsetDateTime, Ordering,
	RankingRequestOverride, ResolvedDiversityPolicy, Result, ScoreCandidateCtx, ScoreSnippetArgs,
	ScoredChunk, SearchFilter, SearchFilterImpact, Uuid, ranking, structured,
};

impl ElfService {
	#[allow(clippy::too_many_arguments)]
	pub(in crate::search) async fn build_finish_search_scoring(
		&self,
		query: &str,
		candidates: Vec<ChunkCandidate>,
		note_meta: &HashMap<Uuid, NoteMeta>,
		policies: &FinishSearchPolicies,
		top_k: u32,
		candidate_count: usize,
		filter: Option<&SearchFilter>,
		requested_candidate_k: u32,
		effective_candidate_k: u32,
		now: OffsetDateTime,
		skip_rerank: bool,
	) -> Result<FinishSearchScoringResult> {
		let (filtered_candidates, filter_impact) = self.apply_filter_to_candidates(
			candidates,
			note_meta,
			filter,
			requested_candidate_k,
			effective_candidate_k,
		);
		let filtered_candidate_count = filtered_candidates.len();
		let snippet_items = self.build_snippet_items(&filtered_candidates, note_meta).await?;
		let snippet_count = snippet_items.len();
		let query_tokens = ranking::tokenize_query(query, MAX_MATCHED_TERMS);
		let scope_context_boost_by_scope =
			ranking::build_scope_context_boost_by_scope(&query_tokens, self.cfg.context.as_ref());
		let det_query_tokens = structured::build_deterministic_query_tokens(&self.cfg, query);
		let scored = self
			.score_snippet_items(ScoreSnippetArgs {
				query,
				snippet_items,
				scope_context_boost_by_scope: &scope_context_boost_by_scope,
				det_query_tokens: det_query_tokens.as_slice(),
				blend_policy: &policies.blend_policy,
				cache_cfg: &self.cfg.search.cache,
				now,
				candidate_count,
				skip_rerank,
			})
			.await?;
		let scored_count = scored.len();
		let trace_candidates = self.build_trace_candidates(&scored, now);
		let results = search::select_best_scored_chunks(scored);
		let fused_results = results.clone();
		let (selected_results, diversity_decisions) =
			self.apply_diversity_policy(results, top_k, &policies.diversity_policy).await?;
		let selected_count = selected_results.len();

		Ok(FinishSearchScoringResult {
			query_tokens,
			filtered_candidates,
			scored_count,
			snippet_count,
			filtered_candidate_count,
			filter_impact,
			trace_candidates,
			fused_results,
			selected_results,
			diversity_decisions,
			selected_count,
		})
	}

	pub(in crate::search) fn apply_filter_to_candidates(
		&self,
		candidates: Vec<ChunkCandidate>,
		note_meta: &HashMap<Uuid, NoteMeta>,
		filter: Option<&SearchFilter>,
		requested_candidate_k: u32,
		effective_candidate_k: u32,
	) -> (Vec<ChunkCandidate>, Option<SearchFilterImpact>) {
		let filtered_candidates: Vec<ChunkCandidate> = candidates
			.into_iter()
			.filter(|candidate| ranking::candidate_matches_note(note_meta, candidate))
			.collect();

		match filter {
			Some(filter) => {
				let (candidates, filter_impact) = filter.eval(
					filtered_candidates,
					note_meta,
					requested_candidate_k,
					effective_candidate_k,
				);

				(candidates, Some(filter_impact))
			},
			None => (filtered_candidates, None),
		}
	}

	pub(in crate::search) fn resolve_finish_search_policies(
		&self,
		ranking_override: Option<&RankingRequestOverride>,
	) -> Result<FinishSearchPolicies> {
		let blend_policy = ranking::resolve_blend_policy(
			&self.cfg.ranking.blend,
			ranking_override.and_then(|override_| override_.blend.as_ref()),
		)?;
		let diversity_policy = ranking::resolve_diversity_policy(
			&self.cfg.ranking.diversity,
			ranking_override.and_then(|override_| override_.diversity.as_ref()),
		)?;
		let retrieval_sources_policy = ranking::resolve_retrieval_sources_policy(
			&self.cfg.ranking.retrieval_sources,
			ranking_override.and_then(|override_| override_.retrieval_sources.as_ref()),
		)?;
		let policy_snapshot = ranking::build_policy_snapshot(
			&self.cfg,
			&blend_policy,
			&diversity_policy,
			&retrieval_sources_policy,
			ranking_override,
		);
		let policy_hash = ranking::hash_policy_snapshot(&policy_snapshot)?;
		let policy_id = format!("ranking_v2:{}", &policy_hash[..12.min(policy_hash.len())]);

		Ok(FinishSearchPolicies {
			blend_policy,
			diversity_policy,
			retrieval_sources_policy,
			policy_snapshot,
			policy_id,
		})
	}

	pub(in crate::search) async fn score_snippet_items(
		&self,
		args: ScoreSnippetArgs<'_, '_>,
	) -> Result<Vec<ScoredChunk>> {
		let ScoreSnippetArgs {
			query,
			snippet_items,
			scope_context_boost_by_scope,
			det_query_tokens,
			blend_policy,
			cache_cfg,
			now,
			candidate_count,
			skip_rerank,
		} = args;

		if snippet_items.is_empty() {
			return Ok(Vec::new());
		}

		let scores = if skip_rerank {
			Self::build_quick_find_rerank_scores(&snippet_items)
		} else {
			self.rerank_snippet_items(query, snippet_items.as_slice(), cache_cfg, now).await?
		};
		let rerank_ranks = ranking::build_rerank_ranks(&snippet_items, &scores);
		let total_rerank = u32::try_from(scores.len()).unwrap_or(1).max(1);
		let total_retrieval = u32::try_from(candidate_count).unwrap_or(1).max(1);
		let score_ctx = ScoreCandidateCtx {
			cfg: &self.cfg,
			blend_policy,
			scope_context_boost_by_scope,
			det_query_tokens,
			now,
			total_rerank,
			total_retrieval,
		};
		let mut scored = Vec::with_capacity(snippet_items.len());

		for ((item, rerank_score), rerank_rank) in
			snippet_items.into_iter().zip(scores).zip(rerank_ranks)
		{
			scored.push(search::score_chunk_candidate(&score_ctx, item, rerank_score, rerank_rank));
		}

		Ok(scored)
	}

	pub(in crate::search) fn build_quick_find_rerank_scores(
		snippet_items: &[ChunkSnippet],
	) -> Vec<f32> {
		let mut idxs: Vec<usize> = (0..snippet_items.len()).collect();

		idxs.sort_by(|&a, &b| {
			let ord = snippet_items[a].retrieval_rank.cmp(&snippet_items[b].retrieval_rank);

			if ord != Ordering::Equal {
				return ord;
			}

			let ord = snippet_items[a].chunk.chunk_index.cmp(&snippet_items[b].chunk.chunk_index);

			if ord != Ordering::Equal {
				return ord;
			}

			snippet_items[a].chunk.chunk_id.cmp(&snippet_items[b].chunk.chunk_id)
		});

		let total = idxs.len();

		if total == 0 {
			return Vec::new();
		}

		let mut scores = vec![0_f32; total];

		for (rank, idx) in idxs.into_iter().enumerate() {
			scores[idx] = 1.0 / (rank as f32 + 1.0);
		}

		scores
	}

	pub(in crate::search) async fn apply_diversity_policy(
		&self,
		results: Vec<ScoredChunk>,
		top_k: u32,
		diversity_policy: &ResolvedDiversityPolicy,
	) -> Result<(Vec<ScoredChunk>, HashMap<Uuid, DiversityDecision>)> {
		let note_vectors = if diversity_policy.enabled {
			search::fetch_note_vectors_for_diversity(&self.db.pool, results.as_slice()).await?
		} else {
			HashMap::new()
		};
		let (selected_results, diversity_decisions) =
			ranking::select_diverse_results(results, top_k, diversity_policy, &note_vectors);

		Ok((selected_results, diversity_decisions))
	}

	pub(in crate::search) async fn record_hits_if_enabled(
		&self,
		enabled: bool,
		query: &str,
		selected_results: &[ScoredChunk],
		now: OffsetDateTime,
	) -> Result<()> {
		if !enabled || selected_results.is_empty() {
			return Ok(());
		}

		let mut tx = self.db.pool.begin().await?;

		search::record_hits(&mut *tx, query, selected_results, now).await?;

		tx.commit().await?;

		Ok(())
	}
}
