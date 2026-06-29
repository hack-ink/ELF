use super::super::*;

pub(in crate::search) struct ScoreSnippetArgs<'a, 'k> {
	pub(in crate::search) query: &'a str,
	pub(in crate::search) snippet_items: Vec<ChunkSnippet>,
	pub(in crate::search) scope_context_boost_by_scope: &'a HashMap<&'k str, f32>,
	pub(in crate::search) det_query_tokens: &'a [String],
	pub(in crate::search) blend_policy: &'a ResolvedBlendPolicy,
	pub(in crate::search) cache_cfg: &'a SearchCache,
	pub(in crate::search) now: OffsetDateTime,
	pub(in crate::search) candidate_count: usize,
	pub(in crate::search) skip_rerank: bool,
}

pub(in crate::search) struct ScoreCandidateCtx<'a, 'k> {
	pub(in crate::search) cfg: &'a Config,
	pub(in crate::search) blend_policy: &'a ResolvedBlendPolicy,
	pub(in crate::search) scope_context_boost_by_scope: &'a HashMap<&'k str, f32>,
	pub(in crate::search) det_query_tokens: &'a [String],
	pub(in crate::search) now: OffsetDateTime,
	pub(in crate::search) total_rerank: u32,
	pub(in crate::search) total_retrieval: u32,
}

#[derive(Clone, Debug)]
pub(in crate::search) struct ScoredChunk {
	pub(in crate::search) item: ChunkSnippet,
	pub(in crate::search) final_score: f32,
	pub(in crate::search) rerank_score: f32,
	pub(in crate::search) rerank_rank: u32,
	pub(in crate::search) rerank_norm: f32,
	pub(in crate::search) retrieval_norm: f32,
	pub(in crate::search) blend_retrieval_weight: f32,
	pub(in crate::search) retrieval_term: f32,
	pub(in crate::search) rerank_term: f32,
	pub(in crate::search) tie_breaker_score: f32,
	pub(in crate::search) scope_context_boost: f32,
	pub(in crate::search) age_days: f32,
	pub(in crate::search) importance: f32,
	pub(in crate::search) deterministic_lexical_overlap_ratio: f32,
	pub(in crate::search) deterministic_lexical_bonus: f32,
	pub(in crate::search) deterministic_hit_count: i64,
	pub(in crate::search) deterministic_last_hit_age_days: Option<f32>,
	pub(in crate::search) deterministic_hit_boost: f32,
	pub(in crate::search) deterministic_decay_penalty: f32,
}

#[derive(Clone, Debug)]
pub(in crate::search) struct DiversityDecision {
	pub(in crate::search) selected: bool,
	pub(in crate::search) selected_rank: Option<u32>,
	pub(in crate::search) selected_reason: String,
	pub(in crate::search) skipped_reason: Option<String>,
	pub(in crate::search) nearest_selected_note_id: Option<Uuid>,
	pub(in crate::search) similarity: Option<f32>,
	pub(in crate::search) mmr_score: Option<f32>,
	pub(in crate::search) missing_embedding: bool,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::search) struct DeterministicRankingTerms {
	pub(in crate::search) lexical_overlap_ratio: f32,
	pub(in crate::search) lexical_bonus: f32,
	pub(in crate::search) hit_count: i64,
	pub(in crate::search) last_hit_age_days: Option<f32>,
	pub(in crate::search) hit_boost: f32,
	pub(in crate::search) decay_penalty: f32,
}
impl Default for DeterministicRankingTerms {
	fn default() -> Self {
		Self {
			lexical_overlap_ratio: 0.0,
			lexical_bonus: 0.0,
			hit_count: 0,
			last_hit_age_days: None,
			hit_boost: 0.0,
			decay_penalty: 0.0,
		}
	}
}

#[derive(Clone, Debug)]
pub(in crate::search) struct ScoredReplay {
	pub(in crate::search) note_id: Uuid,
	pub(in crate::search) chunk_id: Uuid,
	pub(in crate::search) retrieval_rank: u32,
	pub(in crate::search) final_score: f32,
	pub(in crate::search) rerank_score: f32,
	pub(in crate::search) rerank_rank: u32,
	pub(in crate::search) rerank_norm: f32,
	pub(in crate::search) retrieval_norm: f32,
	pub(in crate::search) blend_retrieval_weight: f32,
	pub(in crate::search) retrieval_term: f32,
	pub(in crate::search) rerank_term: f32,
	pub(in crate::search) tie_breaker_score: f32,
	pub(in crate::search) scope_context_boost: f32,
	pub(in crate::search) age_days: f32,
	pub(in crate::search) importance: f32,
	pub(in crate::search) note_scope: String,
	pub(in crate::search) deterministic_lexical_overlap_ratio: f32,
	pub(in crate::search) deterministic_lexical_bonus: f32,
	pub(in crate::search) deterministic_hit_count: i64,
	pub(in crate::search) deterministic_last_hit_age_days: Option<f32>,
	pub(in crate::search) deterministic_hit_boost: f32,
	pub(in crate::search) deterministic_decay_penalty: f32,
}
