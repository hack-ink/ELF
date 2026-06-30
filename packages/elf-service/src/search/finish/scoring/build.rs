use crate::search::{
	self, ChunkCandidate, ElfService, FinishSearchPolicies, FinishSearchScoringResult, HashMap,
	MAX_MATCHED_TERMS, NoteMeta, OffsetDateTime, Result, ScoreSnippetArgs, SearchFilter, Uuid,
	ranking, structured,
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
}
