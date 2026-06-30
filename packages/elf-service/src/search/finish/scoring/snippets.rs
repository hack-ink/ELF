use crate::search::{
	self, ChunkSnippet, ElfService, Ordering, Result, ScoreCandidateCtx, ScoreSnippetArgs,
	ScoredChunk, ranking,
};

impl ElfService {
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
}
