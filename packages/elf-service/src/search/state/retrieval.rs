use super::super::*;

pub(in crate::search) struct MaybeDynamicSearchArgs<'a> {
	pub(in crate::search) path: RawSearchPath,
	pub(in crate::search) enabled: bool,
	pub(in crate::search) trace_id: Uuid,
	pub(in crate::search) query: &'a str,
	pub(in crate::search) tenant_id: &'a str,
	pub(in crate::search) project_id: &'a str,
	pub(in crate::search) agent_id: &'a str,
	pub(in crate::search) token_id: Option<&'a str>,
	pub(in crate::search) read_profile: &'a str,
	pub(in crate::search) allowed_scopes: &'a [String],
	pub(in crate::search) project_context_description: Option<&'a str>,
	pub(in crate::search) filter: &'a Filter,
	pub(in crate::search) service_filter: Option<&'a SearchFilter>,
	pub(in crate::search) candidate_k: u32,
	pub(in crate::search) requested_candidate_k: u32,
	pub(in crate::search) effective_candidate_k: u32,
	pub(in crate::search) top_k: u32,
	pub(in crate::search) record_hits_enabled: bool,
	pub(in crate::search) ranking_override: Option<&'a RankingRequestOverride>,
	pub(in crate::search) retrieval_sources_policy: &'a ResolvedRetrievalSourcesPolicy,
	pub(in crate::search) payload_level: PayloadLevel,
}

pub(in crate::search) struct SearchRetrievalArgs<'a> {
	pub(in crate::search) query: &'a str,
	pub(in crate::search) expansion_mode: ExpansionMode,
	pub(in crate::search) project_context_description: Option<&'a str>,
	pub(in crate::search) filter: &'a Filter,
	pub(in crate::search) candidate_k: u32,
	pub(in crate::search) baseline_vector: Option<&'a Vec<f32>>,
	pub(in crate::search) tenant_id: &'a str,
	pub(in crate::search) project_id: &'a str,
	pub(in crate::search) agent_id: &'a str,
	pub(in crate::search) allowed_scopes: &'a [String],
	pub(in crate::search) retrieval_sources_policy: &'a ResolvedRetrievalSourcesPolicy,
}

pub(in crate::search) struct RecursiveRetrievalArgs<'a> {
	pub(in crate::search) query: &'a str,
	pub(in crate::search) query_vec: &'a [f32],
	pub(in crate::search) filter: &'a Filter,
	pub(in crate::search) candidate_k: u32,
	pub(in crate::search) retrieval_sources_policy: &'a ResolvedRetrievalSourcesPolicy,
	pub(in crate::search) seed_candidates: &'a [ChunkCandidate],
}

pub(in crate::search) struct SearchRetrievalResult {
	pub(in crate::search) expanded_queries: Vec<String>,
	pub(in crate::search) candidates: Vec<ChunkCandidate>,
	pub(in crate::search) structured_matches: HashMap<Uuid, Vec<String>>,
	pub(in crate::search) recursive: Option<RecursiveRetrievalResult>,
}

#[derive(Clone, Debug, Default)]
pub(in crate::search) struct RecursiveRetrievalResult {
	pub(in crate::search) enabled: bool,
	pub(in crate::search) rounds_executed: u32,
	pub(in crate::search) scopes_seeded: usize,
	pub(in crate::search) scopes_queried: usize,
	pub(in crate::search) candidates_before: usize,
	pub(in crate::search) candidates_after: usize,
	pub(in crate::search) candidates_added: usize,
	pub(in crate::search) total_queries: u32,
	pub(in crate::search) stop_reason: Option<String>,
	pub(in crate::search) candidates: Vec<ChunkCandidate>,
}

#[derive(Clone, Debug)]
pub(in crate::search) struct QueryEmbedding {
	pub(in crate::search) text: String,
	pub(in crate::search) vector: Vec<f32>,
}

#[derive(Clone, Debug)]
pub(in crate::search) struct ChunkCandidate {
	pub(in crate::search) chunk_id: Uuid,
	pub(in crate::search) note_id: Uuid,
	pub(in crate::search) chunk_index: i32,
	pub(in crate::search) retrieval_rank: u32,
	pub(in crate::search) retrieval_score: Option<f32>,
	pub(in crate::search) scope: Option<String>,
	pub(in crate::search) updated_at: Option<OffsetDateTime>,
	pub(in crate::search) embedding_version: Option<String>,
}

#[derive(Clone, Debug)]
pub(in crate::search) struct RerankCacheCandidate {
	pub(in crate::search) chunk_id: Uuid,
	pub(in crate::search) updated_at: OffsetDateTime,
}

pub(in crate::search) struct StructuredFieldRetrievalArgs<'a> {
	pub(in crate::search) tenant_id: &'a str,
	pub(in crate::search) project_id: &'a str,
	pub(in crate::search) agent_id: &'a str,
	pub(in crate::search) allowed_scopes: &'a [String],
	pub(in crate::search) query_vec: &'a [f32],
	pub(in crate::search) candidate_k: u32,
	pub(in crate::search) now: OffsetDateTime,
}

#[derive(Debug)]
pub(in crate::search) struct FieldHit {
	pub(in crate::search) note_id: Uuid,
	pub(in crate::search) field_kind: String,
}

pub(in crate::search) struct StructuredFieldHitArgs<'a> {
	pub(in crate::search) embed_version: &'a str,
	pub(in crate::search) tenant_id: &'a str,
	pub(in crate::search) project_id: &'a str,
	pub(in crate::search) agent_id: &'a str,
	pub(in crate::search) now: OffsetDateTime,
	pub(in crate::search) vec_text: &'a str,
	pub(in crate::search) retrieval_limit: i64,
	pub(in crate::search) private_allowed: bool,
	pub(in crate::search) non_private_scopes: &'a [String],
}

#[derive(Clone, Debug)]
pub(in crate::search) struct StructuredFieldRetrievalResult {
	pub(in crate::search) candidates: Vec<ChunkCandidate>,
	pub(in crate::search) structured_matches: HashMap<Uuid, Vec<String>>,
}

#[derive(Clone, Debug)]
pub(in crate::search) struct RetrievalSourceCandidates {
	pub(in crate::search) source: RetrievalSourceKind,
	pub(in crate::search) candidates: Vec<ChunkCandidate>,
}

#[derive(Clone, Debug, Default)]
pub(in crate::search) struct DynamicGateSummary {
	pub(in crate::search) considered: bool,
	pub(in crate::search) should_expand: Option<bool>,
	pub(in crate::search) observed_candidates: Option<u32>,
	pub(in crate::search) observed_top_score: Option<f32>,
}
