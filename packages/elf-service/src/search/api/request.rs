use super::*;

/// Request payload for search APIs.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchRequest {
	/// Tenant to search within.
	pub tenant_id: String,
	/// Project to search within.
	pub project_id: String,
	/// Agent requesting the search.
	pub agent_id: String,
	/// Optional auth token identifier used for role checks.
	pub token_id: Option<String>,
	#[serde(default)]
	/// Requested payload-detail level.
	pub payload_level: PayloadLevel,
	/// Read profile that determines visible scopes.
	pub read_profile: String,
	/// Search query text.
	pub query: String,
	/// Requested number of returned items.
	pub top_k: Option<u32>,
	/// Retrieval breadth before ranking and projection.
	pub candidate_k: Option<u32>,

	/// Optional structured filter expression.
	pub filter: Option<Value>,
	/// When true, records note-hit metrics for returned items.
	pub record_hits: Option<bool>,
	/// Optional ranking-policy overrides.
	pub ranking: Option<RankingRequestOverride>,
}

/// Ranking override bundle supplied on a search request.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RankingRequestOverride {
	/// Blend-ranking override.
	pub blend: Option<BlendRankingOverride>,
	/// Diversity-ranking override.
	pub diversity: Option<DiversityRankingOverride>,
	/// Retrieval-source weighting override.
	pub retrieval_sources: Option<RetrievalSourcesRankingOverride>,
}

/// Blend-ranking override supplied on a search request.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BlendRankingOverride {
	/// Enables or disables blend ranking.
	pub enabled: Option<bool>,
	/// Override for rerank-score normalization.
	pub rerank_normalization: Option<String>,
	/// Override for retrieval-score normalization.
	pub retrieval_normalization: Option<String>,
	/// Override for blend segments.
	pub segments: Option<Vec<BlendSegmentOverride>>,
}

/// One blend segment override.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BlendSegmentOverride {
	/// Highest retrieval rank covered by the segment.
	pub max_retrieval_rank: u32,
	/// Retrieval weight applied within the segment.
	pub retrieval_weight: f32,
}

/// Diversity-ranking override supplied on a search request.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiversityRankingOverride {
	/// Enables or disables diversity selection.
	pub enabled: Option<bool>,
	/// Similarity threshold for duplicate suppression.
	pub sim_threshold: Option<f32>,
	/// MMR lambda value.
	pub mmr_lambda: Option<f32>,
	/// Maximum number of candidates to skip while selecting diverse results.
	pub max_skips: Option<u32>,
}

/// Retrieval-source weighting override supplied on a search request.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RetrievalSourcesRankingOverride {
	/// Weight for fusion retrieval.
	pub fusion_weight: Option<f32>,
	/// Weight for structured-field retrieval.
	pub structured_field_weight: Option<f32>,
	/// Priority for fusion retrieval.
	pub fusion_priority: Option<u32>,
	/// Priority for structured-field retrieval.
	pub structured_field_priority: Option<u32>,
	/// Weight for recursive retrieval.
	pub recursive_weight: Option<f32>,
	/// Priority for recursive retrieval.
	pub recursive_priority: Option<u32>,
}
