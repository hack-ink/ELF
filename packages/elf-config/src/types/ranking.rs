use serde::Deserialize;

/// Ranking settings for retrieval and rerank fusion.
#[derive(Debug, Deserialize)]
pub struct Ranking {
	/// Recency decay window in days.
	pub recency_tau_days: f32,
	/// Small deterministic tie-breaker weight.
	pub tie_breaker_weight: f32,
	/// Retrieval/rerank blending configuration.
	pub blend: RankingBlend,
	/// Optional deterministic scoring overlays.
	pub deterministic: RankingDeterministic,
	/// Diversity settings applied during selection.
	pub diversity: RankingDiversity,
	/// Source weighting and priority between fusion and structured fields.
	pub retrieval_sources: RankingRetrievalSources,
}

/// Deterministic ranking overlays applied on top of model scores.
#[derive(Debug, Deserialize)]
pub struct RankingDeterministic {
	/// Whether deterministic overlays are enabled.
	pub enabled: bool,
	/// Lexical-overlap term settings.
	pub lexical: RankingDeterministicLexical,
	/// Historical-hit term settings.
	pub hits: RankingDeterministicHits,
	/// Decay term settings.
	pub decay: RankingDeterministicDecay,
}

/// Lexical-overlap deterministic term.
#[derive(Debug, Deserialize)]
pub struct RankingDeterministicLexical {
	/// Whether the lexical term is enabled.
	pub enabled: bool,
	/// Weight assigned to the lexical term.
	pub weight: f32,
	/// Minimum overlap ratio required before the term applies.
	pub min_ratio: f32,
	/// Maximum number of query terms examined.
	pub max_query_terms: u32,
	/// Maximum number of text terms examined.
	pub max_text_terms: u32,
}

/// Historical-hit deterministic term.
#[derive(Debug, Deserialize)]
pub struct RankingDeterministicHits {
	/// Whether the hits term is enabled.
	pub enabled: bool,
	/// Weight assigned to the hits term.
	pub weight: f32,
	/// Half-saturation parameter for hit-count scaling.
	pub half_saturation: f32,
	/// Decay window in days for the last-hit component.
	pub last_hit_tau_days: f32,
}

/// Decay-based deterministic term.
#[derive(Debug, Deserialize)]
pub struct RankingDeterministicDecay {
	/// Whether the decay term is enabled.
	pub enabled: bool,
	/// Weight assigned to the decay term.
	pub weight: f32,
	/// Decay window in days.
	pub tau_days: f32,
}

/// Retrieval/rerank blending configuration.
#[derive(Debug, Deserialize)]
pub struct RankingBlend {
	/// Whether blend mode is enabled.
	pub enabled: bool,
	/// Normalization strategy applied to rerank scores.
	pub rerank_normalization: String,
	/// Normalization strategy applied to retrieval scores.
	pub retrieval_normalization: String,
	/// Retrieval-rank segments that assign retrieval weights.
	pub segments: Vec<RankingBlendSegment>,
}

/// One retrieval-rank segment used by blend mode.
#[derive(Debug, Deserialize)]
pub struct RankingBlendSegment {
	/// Inclusive maximum retrieval rank for this segment.
	pub max_retrieval_rank: u32,
	/// Retrieval weight applied within this segment.
	pub retrieval_weight: f32,
}

/// Diversity controls used when selecting final results.
#[derive(Debug, Deserialize)]
pub struct RankingDiversity {
	/// Whether diversity filtering is enabled.
	pub enabled: bool,
	/// Similarity threshold above which candidates may be skipped.
	pub sim_threshold: f32,
	/// Lambda used by MMR-style balancing.
	pub mmr_lambda: f32,
	/// Maximum number of skipped candidates before backfilling.
	pub max_skips: u32,
}

/// Source weighting and priority between fusion and structured-field retrieval.
#[derive(Debug, Deserialize)]
pub struct RankingRetrievalSources {
	/// Weight applied to fused retrieval results.
	pub fusion_weight: f32,
	/// Weight applied to structured-field matches.
	pub structured_field_weight: f32,
	/// Priority assigned to fused retrieval results.
	pub fusion_priority: u32,
	/// Priority assigned to structured-field matches.
	pub structured_field_priority: u32,
}
