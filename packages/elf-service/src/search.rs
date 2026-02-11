use std::{
	cmp::Ordering,
	collections::{BTreeMap, HashMap, HashSet, hash_map::DefaultHasher},
	hash::{Hash, Hasher},
	slice,
};

use qdrant_client::qdrant::{
	Condition, Document, Filter, Fusion, MinShould, PrefetchQueryBuilder, Query,
	QueryPointsBuilder, ScoredPoint, Value, point_id::PointIdOptions, value::Kind,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::{PgExecutor, QueryBuilder};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::{ElfService, Error, Result, ranking_explain_v2};
use elf_config::Config;
use elf_domain::cjk;
use elf_storage::{
	models::MemoryNote,
	qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME},
};

const TRACE_VERSION: i32 = 2;
const MAX_MATCHED_TERMS: usize = 8;
const EXPANSION_CACHE_SCHEMA_VERSION: i32 = 1;
const RERANK_CACHE_SCHEMA_VERSION: i32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExpansionMode {
	Off,
	Always,
	Dynamic,
}

#[derive(Clone, Copy, Debug)]
enum CacheKind {
	Expansion,
	Rerank,
}
impl CacheKind {
	fn as_str(self) -> &'static str {
		match self {
			Self::Expansion => "expansion",
			Self::Rerank => "rerank",
		}
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub read_profile: String,
	pub query: String,
	pub top_k: Option<u32>,
	pub candidate_k: Option<u32>,
	pub record_hits: Option<bool>,
	#[serde(default)]
	pub ranking: Option<RankingRequestOverride>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankingRequestOverride {
	#[serde(default)]
	pub blend: Option<BlendRankingOverride>,
	#[serde(default)]
	pub diversity: Option<DiversityRankingOverride>,
	#[serde(default)]
	pub retrieval_sources: Option<RetrievalSourcesRankingOverride>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlendRankingOverride {
	pub enabled: Option<bool>,
	pub rerank_normalization: Option<String>,
	pub retrieval_normalization: Option<String>,
	pub segments: Option<Vec<BlendSegmentOverride>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlendSegmentOverride {
	pub max_retrieval_rank: u32,
	pub retrieval_weight: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiversityRankingOverride {
	pub enabled: Option<bool>,
	pub sim_threshold: Option<f32>,
	pub mmr_lambda: Option<f32>,
	pub max_skips: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetrievalSourcesRankingOverride {
	pub fusion_weight: Option<f32>,
	pub structured_field_weight: Option<f32>,
	pub fusion_priority: Option<u32>,
	pub structured_field_priority: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchExplain {
	pub r#match: SearchMatchExplain,
	pub ranking: SearchRankingExplain,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub diversity: Option<SearchDiversityExplain>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchMatchExplain {
	pub matched_terms: Vec<String>,
	pub matched_fields: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchDiversityExplain {
	pub enabled: bool,
	pub selected_reason: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub skipped_reason: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub nearest_selected_note_id: Option<Uuid>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub similarity: Option<f32>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub mmr_score: Option<f32>,
	#[serde(default)]
	pub missing_embedding: bool,
}

pub use crate::ranking_explain_v2::{SearchRankingExplain, SearchRankingTerm};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchItem {
	pub result_handle: Uuid,
	pub note_id: Uuid,
	pub chunk_id: Uuid,
	pub chunk_index: i32,
	pub start_offset: i32,
	pub end_offset: i32,
	pub snippet: String,
	pub r#type: String,
	pub key: Option<String>,
	pub scope: String,
	pub importance: f32,
	pub confidence: f32,
	#[serde(with = "crate::time_serde")]
	pub updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	pub expires_at: Option<OffsetDateTime>,
	pub final_score: f32,
	pub source_ref: serde_json::Value,
	pub explain: SearchExplain,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResponse {
	pub trace_id: Uuid,
	pub items: Vec<SearchItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchExplainRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub result_handle: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchTrace {
	pub trace_id: Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub read_profile: String,
	pub query: String,
	pub expansion_mode: String,
	pub expanded_queries: Vec<String>,
	pub allowed_scopes: Vec<String>,
	pub candidate_count: u32,
	pub top_k: u32,
	pub config_snapshot: serde_json::Value,
	#[serde(with = "crate::time_serde")]
	pub created_at: OffsetDateTime,
	pub trace_version: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchExplainItem {
	pub result_handle: Uuid,
	pub note_id: Uuid,
	pub chunk_id: Option<Uuid>,
	pub rank: u32,
	pub explain: SearchExplain,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchExplainResponse {
	pub trace: SearchTrace,
	pub item: SearchExplainItem,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceGetRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub trace_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceGetResponse {
	pub trace: SearchTrace,
	pub items: Vec<SearchExplainItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceReplayContext {
	pub trace_id: Uuid,
	pub query: String,
	pub candidate_count: u32,
	pub top_k: u32,
	#[serde(with = "crate::time_serde")]
	pub created_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceReplayCandidate {
	pub note_id: Uuid,
	pub chunk_id: Uuid,
	pub chunk_index: i32,
	pub snippet: String,
	pub retrieval_rank: u32,
	pub rerank_score: f32,
	pub note_scope: String,
	pub note_importance: f32,
	#[serde(with = "crate::time_serde")]
	pub note_updated_at: OffsetDateTime,
	pub note_hit_count: i64,
	#[serde(with = "crate::time_serde::option")]
	pub note_last_hit_at: Option<OffsetDateTime>,
	#[serde(default)]
	pub diversity_selected: Option<bool>,
	#[serde(default)]
	pub diversity_selected_rank: Option<u32>,
	#[serde(default)]
	pub diversity_selected_reason: Option<String>,
	#[serde(default)]
	pub diversity_skipped_reason: Option<String>,
	#[serde(default)]
	pub diversity_nearest_selected_note_id: Option<Uuid>,
	#[serde(default)]
	pub diversity_similarity: Option<f32>,
	#[serde(default)]
	pub diversity_mmr_score: Option<f32>,
	#[serde(default)]
	pub diversity_missing_embedding: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceReplayItem {
	pub note_id: Uuid,
	pub chunk_id: Uuid,
	pub retrieval_rank: u32,
	pub final_score: f32,
	pub explain: SearchExplain,
}

#[derive(Clone, Debug)]
struct QueryEmbedding {
	text: String,
	vector: Vec<f32>,
}

#[derive(Clone, Debug)]
struct ChunkCandidate {
	chunk_id: Uuid,
	note_id: Uuid,
	chunk_index: i32,
	retrieval_rank: u32,
	updated_at: Option<OffsetDateTime>,
	embedding_version: Option<String>,
}

#[derive(Clone, Debug)]
struct RerankCacheCandidate {
	chunk_id: Uuid,
	updated_at: OffsetDateTime,
}

#[derive(Clone, Debug)]
struct NoteMeta {
	note_id: Uuid,
	note_type: String,
	key: Option<String>,
	scope: String,
	importance: f32,
	confidence: f32,
	updated_at: OffsetDateTime,
	expires_at: Option<OffsetDateTime>,
	source_ref: serde_json::Value,
	embedding_version: String,
	hit_count: i64,
	last_hit_at: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
struct ChunkRow {
	chunk_id: Uuid,
	note_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: String,
}

#[derive(Clone, Debug, sqlx::FromRow)]
struct NoteVectorRow {
	note_id: Uuid,
	vec_text: String,
}

#[derive(Clone, Debug)]
struct ChunkMeta {
	chunk_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
}

#[derive(Clone, Debug)]
struct ChunkSnippet {
	note: NoteMeta,
	chunk: ChunkMeta,
	snippet: String,
	retrieval_rank: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ExpansionCachePayload {
	queries: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpansionOutput {
	queries: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RerankCacheItem {
	chunk_id: Uuid,
	updated_at: OffsetDateTime,
	score: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RerankCachePayload {
	items: Vec<RerankCacheItem>,
}

#[derive(Clone, Debug)]
struct CachePayload {
	value: serde_json::Value,
	size_bytes: usize,
}

#[derive(Clone, Debug)]
struct ScoredChunk {
	item: ChunkSnippet,
	final_score: f32,
	rerank_score: f32,
	rerank_rank: u32,
	rerank_norm: f32,
	retrieval_norm: f32,
	blend_retrieval_weight: f32,
	retrieval_term: f32,
	rerank_term: f32,
	tie_breaker_score: f32,
	scope_context_boost: f32,
	age_days: f32,
	importance: f32,
	deterministic_lexical_overlap_ratio: f32,
	deterministic_lexical_bonus: f32,
	deterministic_hit_count: i64,
	deterministic_last_hit_age_days: Option<f32>,
	deterministic_hit_boost: f32,
	deterministic_decay_penalty: f32,
}

#[derive(Clone, Debug)]
struct DiversityDecision {
	selected: bool,
	selected_rank: Option<u32>,
	selected_reason: String,
	skipped_reason: Option<String>,
	nearest_selected_note_id: Option<Uuid>,
	similarity: Option<f32>,
	mmr_score: Option<f32>,
	missing_embedding: bool,
}

#[derive(Clone, Copy, Debug)]
struct DeterministicRankingTerms {
	lexical_overlap_ratio: f32,
	lexical_bonus: f32,
	hit_count: i64,
	last_hit_age_days: Option<f32>,
	hit_boost: f32,
	decay_penalty: f32,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TracePayload {
	trace: TraceRecord,
	items: Vec<TraceItemRecord>,
	#[serde(default)]
	candidates: Vec<TraceCandidateRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TraceRecord {
	trace_id: Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
	query: String,
	expansion_mode: String,
	expanded_queries: Vec<String>,
	allowed_scopes: Vec<String>,
	candidate_count: u32,
	top_k: u32,
	config_snapshot: serde_json::Value,
	trace_version: i32,
	created_at: OffsetDateTime,
	expires_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TraceItemRecord {
	item_id: Uuid,
	note_id: Uuid,
	chunk_id: Option<Uuid>,
	rank: u32,
	final_score: f32,
	explain: SearchExplain,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TraceCandidateRecord {
	candidate_id: Uuid,
	note_id: Uuid,
	chunk_id: Uuid,
	chunk_index: i32,
	snippet: String,
	#[serde(default)]
	candidate_snapshot: serde_json::Value,
	retrieval_rank: u32,
	rerank_score: f32,
	note_scope: String,
	note_importance: f32,
	note_updated_at: OffsetDateTime,
	note_hit_count: i64,
	note_last_hit_at: Option<OffsetDateTime>,
	created_at: OffsetDateTime,
	expires_at: OffsetDateTime,
}

struct TraceContext<'a> {
	trace_id: Uuid,
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	read_profile: &'a str,
	query: &'a str,
	expansion_mode: ExpansionMode,
	expanded_queries: Vec<String>,
	allowed_scopes: &'a [String],
	candidate_count: usize,
	top_k: u32,
}

struct SearchTraceBuilder {
	trace: TraceRecord,
	items: Vec<TraceItemRecord>,
	candidates: Vec<TraceCandidateRecord>,
}
impl SearchTraceBuilder {
	fn new(
		context: TraceContext<'_>,
		config_snapshot: serde_json::Value,
		retention_days: i64,
		now: OffsetDateTime,
	) -> Self {
		let trace = TraceRecord {
			trace_id: context.trace_id,
			tenant_id: context.tenant_id.to_string(),
			project_id: context.project_id.to_string(),
			agent_id: context.agent_id.to_string(),
			read_profile: context.read_profile.to_string(),
			query: context.query.to_string(),
			expansion_mode: expansion_mode_label(context.expansion_mode).to_string(),
			expanded_queries: context.expanded_queries,
			allowed_scopes: context.allowed_scopes.to_vec(),
			candidate_count: context.candidate_count as u32,
			top_k: context.top_k,
			config_snapshot,
			trace_version: TRACE_VERSION,
			created_at: now,
			expires_at: now + Duration::days(retention_days),
		};
		Self { trace, items: Vec::new(), candidates: Vec::new() }
	}

	fn push_item(&mut self, item: TraceItemRecord) {
		self.items.push(item);
	}

	fn push_candidate(&mut self, candidate: TraceCandidateRecord) {
		self.candidates.push(candidate);
	}

	fn build(self) -> TracePayload {
		TracePayload { trace: self.trace, items: self.items, candidates: self.candidates }
	}
}

struct FinishSearchArgs<'a> {
	trace_id: Uuid,
	query: &'a str,
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	read_profile: &'a str,
	allowed_scopes: &'a [String],
	expanded_queries: Vec<String>,
	expansion_mode: ExpansionMode,
	candidates: Vec<ChunkCandidate>,
	structured_matches: HashMap<Uuid, Vec<String>>,
	top_k: u32,
	record_hits_enabled: bool,
	ranking_override: Option<RankingRequestOverride>,
}

struct StructuredFieldRetrievalArgs<'a> {
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	allowed_scopes: &'a [String],
	query_vec: &'a [f32],
	candidate_k: u32,
	now: OffsetDateTime,
}

#[derive(Clone, Debug)]
struct StructuredFieldRetrievalResult {
	candidates: Vec<ChunkCandidate>,
	structured_matches: HashMap<Uuid, Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RetrievalSourceKind {
	Fusion,
	StructuredField,
}

#[derive(Debug, Clone)]
struct RetrievalSourceCandidates {
	source: RetrievalSourceKind,
	candidates: Vec<ChunkCandidate>,
}

impl ElfService {
	pub async fn search_raw(&self, req: SearchRequest) -> Result<SearchResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}
		if cjk::contains_cjk(&req.query) {
			return Err(Error::NonEnglishInput { field: "$.query".to_string() });
		}

		let top_k = req.top_k.unwrap_or(self.cfg.memory.top_k).max(1);
		let candidate_k = req.candidate_k.unwrap_or(self.cfg.memory.candidate_k).max(top_k);
		let query = req.query.clone();
		let read_profile = req.read_profile.clone();
		let record_hits_enabled = req.record_hits.unwrap_or(false);
		let ranking_override = req.ranking.clone();
		let retrieval_sources_policy = resolve_retrieval_sources_policy(
			&self.cfg.ranking.retrieval_sources,
			ranking_override.as_ref().and_then(|override_| override_.retrieval_sources.as_ref()),
		)?;
		let expansion_mode = resolve_expansion_mode(&self.cfg);
		let trace_id = Uuid::new_v4();
		let project_context_description =
			self.resolve_project_context_description(tenant_id, project_id);
		let allowed_scopes = resolve_scopes(&self.cfg, &read_profile)?;

		if allowed_scopes.is_empty() {
			return self
				.finish_search(FinishSearchArgs {
					trace_id,
					query: &query,
					tenant_id,
					project_id,
					agent_id,
					read_profile: &read_profile,
					allowed_scopes: &allowed_scopes,
					expanded_queries: vec![query.clone()],
					expansion_mode,
					candidates: Vec::new(),
					structured_matches: HashMap::new(),
					top_k,
					record_hits_enabled,
					ranking_override: ranking_override.clone(),
				})
				.await;
		}

		let private_scope = "agent_private".to_string();
		let non_private_scopes: Vec<String> =
			allowed_scopes.iter().filter(|scope| *scope != "agent_private").cloned().collect();
		let mut should_conditions = Vec::new();

		if allowed_scopes.iter().any(|scope| scope == "agent_private") {
			let private_filter = Filter::all([
				Condition::matches("scope", private_scope),
				Condition::matches("agent_id", agent_id.to_string()),
			]);

			should_conditions.push(Condition::from(private_filter));
		}
		if !non_private_scopes.is_empty() {
			should_conditions.push(Condition::matches("scope", non_private_scopes));
		}

		let (should, min_should) = if should_conditions.is_empty() {
			(Vec::new(), None)
		} else {
			(Vec::new(), Some(MinShould { min_count: 1, conditions: should_conditions }))
		};
		let filter = Filter {
			must: vec![
				Condition::matches("tenant_id", tenant_id.to_string()),
				Condition::matches("project_id", project_id.to_string()),
				Condition::matches("status", "active".to_string()),
			],
			should,
			must_not: Vec::new(),
			min_should,
		};
		let mut baseline_vector: Option<Vec<f32>> = None;

		if expansion_mode == ExpansionMode::Dynamic {
			let query_vec = self.embed_single_query(&query, project_context_description).await?;

			baseline_vector = Some(query_vec.clone());

			let baseline_points = self
				.run_fusion_query(
					&[QueryEmbedding { text: query.clone(), vector: query_vec.clone() }],
					&filter,
					candidate_k,
				)
				.await?;
			let top_score = baseline_points.first().map(|point| point.score).unwrap_or(0.0);
			let candidates = collect_chunk_candidates(
				&baseline_points,
				self.cfg.search.prefilter.max_candidates,
				candidate_k,
			);
			let should_expand =
				should_expand_dynamic(baseline_points.len(), top_score, &self.cfg.search.dynamic);

			if !should_expand {
				let structured = self
					.retrieve_structured_field_candidates(StructuredFieldRetrievalArgs {
						tenant_id,
						project_id,
						agent_id,
						allowed_scopes: &allowed_scopes,
						query_vec: query_vec.as_slice(),
						candidate_k,
						now: OffsetDateTime::now_utc(),
					})
					.await?;
				let merged_candidates = merge_retrieval_candidates(
					vec![
						RetrievalSourceCandidates {
							source: RetrievalSourceKind::Fusion,
							candidates,
						},
						RetrievalSourceCandidates {
							source: RetrievalSourceKind::StructuredField,
							candidates: structured.candidates,
						},
					],
					&retrieval_sources_policy,
					candidate_k,
				);

				return self
					.finish_search(FinishSearchArgs {
						trace_id,
						query: &query,
						tenant_id,
						project_id,
						agent_id,
						read_profile: &read_profile,
						allowed_scopes: &allowed_scopes,
						expanded_queries: vec![query.clone()],
						expansion_mode,
						candidates: merged_candidates,
						structured_matches: structured.structured_matches,
						top_k,
						record_hits_enabled,
						ranking_override: ranking_override.clone(),
					})
					.await;
			}
		}

		let queries = match expansion_mode {
			ExpansionMode::Off => vec![query.clone()],
			ExpansionMode::Always | ExpansionMode::Dynamic => self.expand_queries(&query).await,
		};
		let expanded_queries = queries.clone();
		let query_embeddings = self
			.embed_queries(&queries, &query, baseline_vector.as_ref(), project_context_description)
			.await?;
		let fusion_points = self.run_fusion_query(&query_embeddings, &filter, candidate_k).await?;
		let candidates = collect_chunk_candidates(
			&fusion_points,
			self.cfg.search.prefilter.max_candidates,
			candidate_k,
		);
		let original_query_vec = query_embeddings
			.iter()
			.find(|embedded| embedded.text == query)
			.map(|embedded| embedded.vector.clone())
			.unwrap_or_else(Vec::new);
		let original_query_vec = if original_query_vec.is_empty() {
			self.embed_single_query(&query, project_context_description).await?
		} else {
			original_query_vec
		};
		let structured = self
			.retrieve_structured_field_candidates(StructuredFieldRetrievalArgs {
				tenant_id,
				project_id,
				agent_id,
				allowed_scopes: &allowed_scopes,
				query_vec: original_query_vec.as_slice(),
				candidate_k,
				now: OffsetDateTime::now_utc(),
			})
			.await?;
		let merged_candidates = merge_retrieval_candidates(
			vec![
				RetrievalSourceCandidates { source: RetrievalSourceKind::Fusion, candidates },
				RetrievalSourceCandidates {
					source: RetrievalSourceKind::StructuredField,
					candidates: structured.candidates,
				},
			],
			&retrieval_sources_policy,
			candidate_k,
		);

		self.finish_search(FinishSearchArgs {
			trace_id,
			query: &query,
			tenant_id,
			project_id,
			agent_id,
			read_profile: &read_profile,
			allowed_scopes: &allowed_scopes,
			expanded_queries,
			expansion_mode,
			candidates: merged_candidates,
			structured_matches: structured.structured_matches,
			top_k,
			record_hits_enabled,
			ranking_override,
		})
		.await
	}

	fn resolve_project_context_description<'a>(
		&'a self,
		tenant_id: &str,
		project_id: &str,
	) -> Option<&'a str> {
		let context = self.cfg.context.as_ref()?;
		let descriptions = context.project_descriptions.as_ref()?;
		let key = format!("{tenant_id}:{project_id}");
		let mut saw_cjk = false;

		if let Some(value) = descriptions.get(&key) {
			let trimmed = value.trim();

			if !trimmed.is_empty() {
				if cjk::contains_cjk(trimmed) {
					saw_cjk = true;
				} else {
					return Some(trimmed);
				}
			}
		}
		if let Some(value) = descriptions.get(project_id) {
			let trimmed = value.trim();

			if !trimmed.is_empty() {
				if cjk::contains_cjk(trimmed) {
					saw_cjk = true;
				} else {
					return Some(trimmed);
				}
			}
		}

		if saw_cjk {
			tracing::warn!(
				tenant_id,
				project_id,
				"Project context description contains CJK. Skipping context."
			);
		}

		None
	}

	pub async fn search_explain(&self, req: SearchExplainRequest) -> Result<SearchExplainResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let row = sqlx::query!(
			"\
SELECT
	t.trace_id AS \"trace_id!\",
	t.tenant_id AS \"tenant_id!\",
	t.project_id AS \"project_id!\",
	t.agent_id AS \"agent_id!\",
	t.read_profile AS \"read_profile!\",
	t.query AS \"query!\",
	t.expansion_mode AS \"expansion_mode!\",
	t.expanded_queries AS \"expanded_queries!\",
	t.allowed_scopes AS \"allowed_scopes!\",
	t.candidate_count AS \"candidate_count!\",
	t.top_k AS \"top_k!\",
	t.config_snapshot AS \"config_snapshot!\",
	t.trace_version AS \"trace_version!\",
	t.created_at AS \"created_at!\",
	i.item_id AS \"item_id!\",
	i.note_id AS \"note_id!\",
	i.chunk_id,
	i.rank AS \"rank!\",
	i.final_score AS \"final_score!\",
	i.explain AS \"explain!\"
FROM search_trace_items i
JOIN search_traces t ON i.trace_id = t.trace_id
WHERE i.item_id = $1 AND t.tenant_id = $2 AND t.project_id = $3 AND t.agent_id = $4",
			req.result_handle,
			tenant_id,
			project_id,
			agent_id,
		)
		.fetch_optional(&self.db.pool)
		.await?;
		let Some(row) = row else {
			return Err(Error::InvalidRequest {
				message: "Unknown result_handle or trace not yet persisted.".to_string(),
			});
		};
		let expanded_queries: Vec<String> = decode_json(row.expanded_queries, "expanded_queries")?;
		let allowed_scopes: Vec<String> = decode_json(row.allowed_scopes, "allowed_scopes")?;
		let config_snapshot = row.config_snapshot;
		let explain: SearchExplain = decode_json(row.explain, "explain")?;
		let trace = SearchTrace {
			trace_id: row.trace_id,
			tenant_id: row.tenant_id,
			project_id: row.project_id,
			agent_id: row.agent_id,
			read_profile: row.read_profile,
			query: row.query,
			expansion_mode: row.expansion_mode,
			expanded_queries,
			allowed_scopes,
			candidate_count: row.candidate_count as u32,
			top_k: row.top_k as u32,
			config_snapshot,
			created_at: row.created_at,
			trace_version: row.trace_version,
		};
		let item = SearchExplainItem {
			result_handle: row.item_id,
			note_id: row.note_id,
			chunk_id: row.chunk_id,
			rank: row.rank as u32,
			explain,
		};

		Ok(SearchExplainResponse { trace, item })
	}

	pub async fn trace_get(&self, req: TraceGetRequest) -> Result<TraceGetResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let row = sqlx::query!(
			"\
SELECT
	trace_id AS \"trace_id!\",
	tenant_id AS \"tenant_id!\",
	project_id AS \"project_id!\",
	agent_id AS \"agent_id!\",
	read_profile AS \"read_profile!\",
	query AS \"query!\",
	expansion_mode AS \"expansion_mode!\",
	expanded_queries AS \"expanded_queries!\",
	allowed_scopes AS \"allowed_scopes!\",
	candidate_count AS \"candidate_count!\",
	top_k AS \"top_k!\",
	config_snapshot AS \"config_snapshot!\",
	trace_version AS \"trace_version!\",
	created_at AS \"created_at!\"
FROM search_traces
WHERE trace_id = $1 AND tenant_id = $2 AND project_id = $3 AND agent_id = $4",
			req.trace_id,
			tenant_id,
			project_id,
			agent_id,
		)
		.fetch_optional(&self.db.pool)
		.await?;
		let Some(row) = row else {
			return Err(Error::InvalidRequest { message: "Unknown trace_id.".to_string() });
		};
		let expanded_queries: Vec<String> = decode_json(row.expanded_queries, "expanded_queries")?;
		let allowed_scopes: Vec<String> = decode_json(row.allowed_scopes, "allowed_scopes")?;
		let config_snapshot = row.config_snapshot;
		let trace = SearchTrace {
			trace_id: row.trace_id,
			tenant_id: row.tenant_id,
			project_id: row.project_id,
			agent_id: row.agent_id,
			read_profile: row.read_profile,
			query: row.query,
			expansion_mode: row.expansion_mode,
			expanded_queries,
			allowed_scopes,
			candidate_count: row.candidate_count as u32,
			top_k: row.top_k as u32,
			config_snapshot,
			created_at: row.created_at,
			trace_version: row.trace_version,
		};
		let item_rows = sqlx::query!(
			"\
SELECT
	item_id AS \"item_id!\",
	note_id AS \"note_id!\",
	chunk_id,
	rank AS \"rank!\",
	final_score AS \"final_score!\",
	explain AS \"explain!\"
FROM search_trace_items
WHERE trace_id = $1
ORDER BY rank ASC",
			req.trace_id,
		)
		.fetch_all(&self.db.pool)
		.await?;
		let mut items = Vec::with_capacity(item_rows.len());

		for row in item_rows {
			let explain: SearchExplain = decode_json(row.explain, "explain")?;

			items.push(SearchExplainItem {
				result_handle: row.item_id,
				note_id: row.note_id,
				chunk_id: row.chunk_id,
				rank: row.rank as u32,
				explain,
			});
		}

		Ok(TraceGetResponse { trace, items })
	}

	async fn embed_single_query(
		&self,
		query: &str,
		project_context_description: Option<&str>,
	) -> Result<Vec<f32>> {
		let input = build_dense_embedding_input(query, project_context_description);
		let embeddings = self
			.providers
			.embedding
			.embed(&self.cfg.providers.embedding, slice::from_ref(&input))
			.await?;
		let query_vec = embeddings.into_iter().next().ok_or_else(|| Error::Provider {
			message: "Embedding provider returned no vectors.".to_string(),
		})?;

		if query_vec.len() != self.cfg.storage.qdrant.vector_dim as usize {
			return Err(Error::Provider {
				message: "Embedding vector dimension mismatch.".to_string(),
			});
		}

		Ok(query_vec)
	}

	async fn embed_queries(
		&self,
		queries: &[String],
		original_query: &str,
		baseline_vector: Option<&Vec<f32>>,
		project_context_description: Option<&str>,
	) -> Result<Vec<QueryEmbedding>> {
		let mut extra_queries = Vec::new();
		let mut extra_inputs = Vec::new();

		for query in queries {
			if baseline_vector.is_some() && query == original_query {
				continue;
			}
			extra_queries.push(query.clone());
			extra_inputs.push(build_dense_embedding_input(query, project_context_description));
		}

		let mut embedded_iter = if extra_queries.is_empty() {
			Vec::new().into_iter()
		} else {
			let embedded = self
				.providers
				.embedding
				.embed(&self.cfg.providers.embedding, &extra_inputs)
				.await?;

			if embedded.len() != extra_queries.len() {
				return Err(Error::Provider {
					message: "Embedding provider returned mismatched vector count.".to_string(),
				});
			}
			embedded.into_iter()
		};
		let mut out = Vec::with_capacity(queries.len());

		for query in queries {
			let vector = if baseline_vector.is_some() && query == original_query {
				baseline_vector
					.ok_or_else(|| Error::Provider {
						message: "Embedding baseline vector is missing.".to_string(),
					})?
					.clone()
			} else {
				embedded_iter.next().ok_or_else(|| Error::Provider {
					message: "Embedding provider returned no vectors.".to_string(),
				})?
			};

			if vector.len() != self.cfg.storage.qdrant.vector_dim as usize {
				return Err(Error::Provider {
					message: "Embedding vector dimension mismatch.".to_string(),
				});
			}
			out.push(QueryEmbedding { text: query.clone(), vector });
		}
		Ok(out)
	}

	async fn run_fusion_query(
		&self,
		queries: &[QueryEmbedding],
		filter: &Filter,
		candidate_k: u32,
	) -> Result<Vec<ScoredPoint>> {
		let mut search = QueryPointsBuilder::new(self.qdrant.collection.clone());

		for query in queries {
			let dense_prefetch = PrefetchQueryBuilder::default()
				.query(Query::new_nearest(query.vector.clone()))
				.using(DENSE_VECTOR_NAME)
				.filter(filter.clone())
				.limit(candidate_k as u64);
			let bm25_prefetch = PrefetchQueryBuilder::default()
				.query(Query::new_nearest(Document::new(query.text.clone(), BM25_MODEL)))
				.using(BM25_VECTOR_NAME)
				.filter(filter.clone())
				.limit(candidate_k as u64);

			search = search.add_prefetch(dense_prefetch).add_prefetch(bm25_prefetch);
		}

		let search = search.with_payload(true).query(Fusion::Rrf).limit(candidate_k as u64);
		let response = self
			.qdrant
			.client
			.query(search)
			.await
			.map_err(|err| Error::Qdrant { message: err.to_string() })?;

		Ok(response.result)
	}

	async fn expand_queries(&self, query: &str) -> Vec<String> {
		let cfg = &self.cfg.search.expansion;
		let cache_cfg = &self.cfg.search.cache;
		let now = OffsetDateTime::now_utc();
		let cache_key = if cache_cfg.enabled {
			match build_expansion_cache_key(
				query,
				cfg.max_queries,
				cfg.include_original,
				self.cfg.providers.llm_extractor.provider_id.as_str(),
				self.cfg.providers.llm_extractor.model.as_str(),
				self.cfg.providers.llm_extractor.temperature,
			) {
				Ok(key) => Some(key),
				Err(err) => {
					tracing::warn!(
						error = %err,
						cache_kind = CacheKind::Expansion.as_str(),
						"Cache key build failed."
					);
					None
				},
			}
		} else {
			None
		};

		if let Some(key) = cache_key.as_ref() {
			match fetch_cache_payload(&self.db.pool, CacheKind::Expansion, key, now).await {
				Ok(Some(payload)) => {
					tracing::info!(
						cache_kind = CacheKind::Expansion.as_str(),
						cache_key_prefix = cache_key_prefix(key),
						hit = true,
						payload_size = payload.size_bytes,
						ttl_days = cache_cfg.expansion_ttl_days,
						"Cache hit."
					);
					let cached: ExpansionCachePayload = match serde_json::from_value(payload.value)
					{
						Ok(value) => value,
						Err(err) => {
							tracing::warn!(
								error = %err,
								cache_kind = CacheKind::Expansion.as_str(),
								cache_key_prefix = cache_key_prefix(key),
								"Cache payload decode failed."
							);
							ExpansionCachePayload { queries: Vec::new() }
						},
					};

					if !cached.queries.is_empty() {
						return cached.queries;
					}
				},
				Ok(None) => {
					tracing::info!(
						cache_kind = CacheKind::Expansion.as_str(),
						cache_key_prefix = cache_key_prefix(key),
						hit = false,
						payload_size = 0_u64,
						ttl_days = cache_cfg.expansion_ttl_days,
						"Cache miss."
					);
				},
				Err(err) => {
					tracing::warn!(
						error = %err,
						cache_kind = CacheKind::Expansion.as_str(),
						cache_key_prefix = cache_key_prefix(key),
						"Cache read failed."
					);
				},
			}
		}

		let messages = build_expansion_messages(query, cfg.max_queries, cfg.include_original);
		let raw = match self
			.providers
			.extractor
			.extract(&self.cfg.providers.llm_extractor, &messages)
			.await
		{
			Ok(value) => value,
			Err(err) => {
				tracing::warn!(error = %err, "Query expansion failed; falling back to original query.");

				return vec![query.to_string()];
			},
		};
		let parsed: ExpansionOutput = match serde_json::from_value(raw) {
			Ok(value) => value,
			Err(err) => {
				tracing::warn!(error = %err, "Query expansion returned invalid JSON; falling back to original query.");

				return vec![query.to_string()];
			},
		};

		let normalized =
			normalize_queries(parsed.queries, query, cfg.include_original, cfg.max_queries);
		let result = if normalized.is_empty() { vec![query.to_string()] } else { normalized };

		if let Some(key) = cache_key {
			let payload = ExpansionCachePayload { queries: result.clone() };
			let payload_json = match serde_json::to_value(&payload) {
				Ok(value) => value,
				Err(err) => {
					tracing::warn!(
						error = %err,
						cache_kind = CacheKind::Expansion.as_str(),
						cache_key_prefix = cache_key_prefix(&key),
						"Cache payload encode failed."
					);

					return result;
				},
			};
			let stored_at = OffsetDateTime::now_utc();
			let expires_at = stored_at + Duration::days(cache_cfg.expansion_ttl_days);

			match store_cache_payload(
				&self.db.pool,
				CacheKind::Expansion,
				&key,
				payload_json,
				stored_at,
				expires_at,
				cache_cfg.max_payload_bytes,
			)
			.await
			{
				Ok(Some(payload_size)) => {
					tracing::info!(
						cache_kind = CacheKind::Expansion.as_str(),
						cache_key_prefix = cache_key_prefix(&key),
						hit = false,
						payload_size,
						ttl_days = cache_cfg.expansion_ttl_days,
						"Cache stored."
					);
				},
				Ok(None) => {
					tracing::warn!(
						cache_kind = CacheKind::Expansion.as_str(),
						cache_key_prefix = cache_key_prefix(&key),
						hit = false,
						payload_size = 0_u64,
						ttl_days = cache_cfg.expansion_ttl_days,
						"Cache payload skipped due to size."
					);
				},
				Err(err) => {
					tracing::warn!(
						error = %err,
						cache_kind = CacheKind::Expansion.as_str(),
						cache_key_prefix = cache_key_prefix(&key),
						"Cache write failed."
					);
				},
			}
		}

		result
	}

	async fn retrieve_structured_field_candidates(
		&self,
		args: StructuredFieldRetrievalArgs<'_>,
	) -> Result<StructuredFieldRetrievalResult> {
		#[derive(Debug)]
		struct FieldHit {
			note_id: Uuid,
			field_kind: String,
		}

		let StructuredFieldRetrievalArgs {
			tenant_id,
			project_id,
			agent_id,
			allowed_scopes,
			query_vec,
			candidate_k,
			now,
		} = args;

		if query_vec.is_empty() {
			return Ok(StructuredFieldRetrievalResult {
				candidates: Vec::new(),
				structured_matches: HashMap::new(),
			});
		}

		let embed_version = crate::embedding_version(&self.cfg);
		let vec_text = crate::vector_to_pg(query_vec);
		let private_allowed = allowed_scopes.iter().any(|scope| scope == "agent_private");
		let non_private_scopes: Vec<String> =
			allowed_scopes.iter().filter(|scope| *scope != "agent_private").cloned().collect();
		let retrieval_limit = i64::from(candidate_k.saturating_mul(4).clamp(16, 400));
		let rows: Vec<FieldHit> = if private_allowed && non_private_scopes.is_empty() {
			let raw = sqlx::query!(
				"\
SELECT
	f.note_id AS \"note_id!\",
	f.field_kind AS \"field_kind!\"
FROM memory_note_fields f
JOIN note_field_embeddings e
	ON e.field_id = f.field_id
	AND e.embedding_version = $1
JOIN memory_notes n
	ON n.note_id = f.note_id
WHERE n.tenant_id = $2
	AND n.project_id = $3
	AND n.status = 'active'
	AND (n.expires_at IS NULL OR n.expires_at > $4)
	AND n.scope = 'agent_private'
	AND n.agent_id = $5
ORDER BY e.vec <=> $6::text::vector ASC
LIMIT $7",
				embed_version,
				tenant_id,
				project_id,
				now,
				agent_id,
				vec_text.as_str(),
				retrieval_limit,
			)
			.fetch_all(&self.db.pool)
			.await?;

			raw.into_iter()
				.map(|row| FieldHit { note_id: row.note_id, field_kind: row.field_kind })
				.collect()
		} else if !private_allowed {
			let raw = sqlx::query!(
				"\
SELECT
	f.note_id AS \"note_id!\",
	f.field_kind AS \"field_kind!\"
FROM memory_note_fields f
JOIN note_field_embeddings e
	ON e.field_id = f.field_id
	AND e.embedding_version = $1
JOIN memory_notes n
	ON n.note_id = f.note_id
WHERE n.tenant_id = $2
	AND n.project_id = $3
	AND n.status = 'active'
	AND (n.expires_at IS NULL OR n.expires_at > $4)
	AND n.scope = ANY($5::text[])
ORDER BY e.vec <=> $6::text::vector ASC
LIMIT $7",
				embed_version,
				tenant_id,
				project_id,
				now,
				non_private_scopes.as_slice(),
				vec_text.as_str(),
				retrieval_limit,
			)
			.fetch_all(&self.db.pool)
			.await?;

			raw.into_iter()
				.map(|row| FieldHit { note_id: row.note_id, field_kind: row.field_kind })
				.collect()
		} else {
			let raw = sqlx::query!(
				"\
SELECT
	f.note_id AS \"note_id!\",
	f.field_kind AS \"field_kind!\"
FROM memory_note_fields f
JOIN note_field_embeddings e
	ON e.field_id = f.field_id
	AND e.embedding_version = $1
JOIN memory_notes n
	ON n.note_id = f.note_id
WHERE n.tenant_id = $2
	AND n.project_id = $3
	AND n.status = 'active'
	AND (n.expires_at IS NULL OR n.expires_at > $4)
	AND (
		(n.scope = 'agent_private' AND n.agent_id = $5)
		OR n.scope = ANY($6::text[])
	)
ORDER BY e.vec <=> $7::text::vector ASC
LIMIT $8",
				embed_version,
				tenant_id,
				project_id,
				now,
				agent_id,
				non_private_scopes.as_slice(),
				vec_text.as_str(),
				retrieval_limit,
			)
			.fetch_all(&self.db.pool)
			.await?;

			raw.into_iter()
				.map(|row| FieldHit { note_id: row.note_id, field_kind: row.field_kind })
				.collect()
		};

		let mut structured_matches: HashMap<Uuid, HashSet<String>> = HashMap::new();
		let mut ordered_note_ids = Vec::new();
		let mut seen_notes = HashSet::new();

		for row in rows {
			let label = match row.field_kind.as_str() {
				"summary" => "summary",
				"fact" => "facts",
				"concept" => "concepts",
				_ => continue,
			};

			structured_matches.entry(row.note_id).or_default().insert(label.to_string());

			if seen_notes.insert(row.note_id) {
				ordered_note_ids.push(row.note_id);
			}
		}

		let mut structured_matches_out: HashMap<Uuid, Vec<String>> = HashMap::new();

		for (note_id, fields) in structured_matches {
			let mut fields: Vec<String> = fields.into_iter().collect();

			fields.sort();
			structured_matches_out.insert(note_id, fields);
		}

		if ordered_note_ids.is_empty() {
			return Ok(StructuredFieldRetrievalResult {
				candidates: Vec::new(),
				structured_matches: structured_matches_out,
			});
		}

		let best_chunks = sqlx::query!(
			"\
SELECT DISTINCT ON (c.note_id)
	c.note_id AS \"note_id!\",
	c.chunk_id AS \"chunk_id!\",
	c.chunk_index AS \"chunk_index!\"
FROM memory_note_chunks c
JOIN note_chunk_embeddings e
	ON e.chunk_id = c.chunk_id
	AND e.embedding_version = $1
WHERE c.note_id = ANY($2::uuid[])
ORDER BY c.note_id ASC, e.vec <=> $3::text::vector ASC",
			embed_version,
			ordered_note_ids.as_slice(),
			vec_text.as_str(),
		)
		.fetch_all(&self.db.pool)
		.await?;
		let mut best_by_note = HashMap::new();

		for row in best_chunks {
			best_by_note.insert(row.note_id, (row.chunk_id, row.chunk_index));
		}

		let mut structured_candidates = Vec::new();
		let mut next_rank = 1_u32;

		for note_id in ordered_note_ids {
			if structured_candidates.len() >= candidate_k as usize {
				break;
			}

			let Some((chunk_id, chunk_index)) = best_by_note.get(&note_id) else { continue };

			structured_candidates.push(ChunkCandidate {
				chunk_id: *chunk_id,
				note_id,
				chunk_index: *chunk_index,
				retrieval_rank: next_rank,
				updated_at: None,
				embedding_version: Some(embed_version.clone()),
			});

			next_rank = next_rank.saturating_add(1);
		}

		Ok(StructuredFieldRetrievalResult {
			candidates: structured_candidates,
			structured_matches: structured_matches_out,
		})
	}

	async fn finish_search(&self, args: FinishSearchArgs<'_>) -> Result<SearchResponse> {
		let FinishSearchArgs {
			trace_id,
			query,
			tenant_id,
			project_id,
			agent_id,
			read_profile,
			allowed_scopes,
			expanded_queries,
			expansion_mode,
			candidates,
			structured_matches,
			top_k,
			record_hits_enabled,
			ranking_override,
		} = args;
		let now = OffsetDateTime::now_utc();
		let cache_cfg = &self.cfg.search.cache;
		let candidate_count = candidates.len();
		let candidate_note_ids: Vec<Uuid> =
			candidates.iter().map(|candidate| candidate.note_id).collect();
		let mut notes: Vec<MemoryNote> = if candidate_note_ids.is_empty() {
			Vec::new()
		} else {
			sqlx::query_as!(
					MemoryNote,
					"SELECT * FROM memory_notes WHERE note_id = ANY($1::uuid[]) AND tenant_id = $2 AND project_id = $3",
					candidate_note_ids.as_slice(),
					tenant_id,
					project_id,
				)
				.fetch_all(&self.db.pool)
				.await?
		};
		let mut note_meta = HashMap::new();

		for note in notes.drain(..) {
			if note.tenant_id != tenant_id || note.project_id != project_id {
				continue;
			}
			if note.scope == "agent_private" && note.agent_id != agent_id {
				continue;
			}
			if note.status != "active" {
				continue;
			}
			if !allowed_scopes.contains(&note.scope) {
				continue;
			}
			if note.expires_at.map(|ts| ts <= now).unwrap_or(false) {
				continue;
			}

			note_meta.insert(
				note.note_id,
				NoteMeta {
					note_id: note.note_id,
					note_type: note.r#type,
					key: note.key,
					scope: note.scope,
					importance: note.importance,
					confidence: note.confidence,
					updated_at: note.updated_at,
					expires_at: note.expires_at,
					source_ref: note.source_ref,
					embedding_version: note.embedding_version,
					hit_count: note.hit_count,
					last_hit_at: note.last_hit_at,
				},
			);
		}

		let filtered_candidates: Vec<ChunkCandidate> = candidates
			.into_iter()
			.filter(|candidate| candidate_matches_note(&note_meta, candidate))
			.collect();
		let snippet_items = if filtered_candidates.is_empty() {
			Vec::new()
		} else {
			let pairs = collect_neighbor_pairs(&filtered_candidates);
			let chunk_rows = fetch_chunks_by_pair(&self.db.pool, &pairs).await?;
			let mut chunk_by_id = HashMap::new();
			let mut chunk_by_note_index = HashMap::new();

			for row in chunk_rows {
				chunk_by_note_index.insert((row.note_id, row.chunk_index), row.clone());
				chunk_by_id.insert(row.chunk_id, row);
			}

			let mut items = Vec::new();

			for candidate in &filtered_candidates {
				let Some(chunk_row) = chunk_by_id.get(&candidate.chunk_id) else {
					tracing::warn!(
						chunk_id = %candidate.chunk_id,
						"Chunk metadata missing for candidate."
					);

					continue;
				};
				let snippet =
					stitch_snippet(candidate.note_id, chunk_row.chunk_index, &chunk_by_note_index);

				if snippet.is_empty() {
					continue;
				}

				let Some(note) = note_meta.get(&candidate.note_id) else { continue };
				let chunk = ChunkMeta {
					chunk_id: chunk_row.chunk_id,
					chunk_index: chunk_row.chunk_index,
					start_offset: chunk_row.start_offset,
					end_offset: chunk_row.end_offset,
				};

				items.push(ChunkSnippet {
					note: note.clone(),
					chunk,
					snippet,
					retrieval_rank: candidate.retrieval_rank,
				});
			}

			items
		};
		let query_tokens = tokenize_query(query, MAX_MATCHED_TERMS);
		let scope_context_boost_by_scope =
			build_scope_context_boost_by_scope(&query_tokens, self.cfg.context.as_ref());
		let det_query_tokens = if self.cfg.ranking.deterministic.enabled
			&& self.cfg.ranking.deterministic.lexical.enabled
			&& self.cfg.ranking.deterministic.lexical.max_query_terms > 0
		{
			tokenize_query(query, self.cfg.ranking.deterministic.lexical.max_query_terms as usize)
		} else {
			Vec::new()
		};
		let blend_policy = resolve_blend_policy(
			&self.cfg.ranking.blend,
			ranking_override.as_ref().and_then(|override_| override_.blend.as_ref()),
		)?;
		let diversity_policy = resolve_diversity_policy(
			&self.cfg.ranking.diversity,
			ranking_override.as_ref().and_then(|override_| override_.diversity.as_ref()),
		)?;
		let retrieval_sources_policy = resolve_retrieval_sources_policy(
			&self.cfg.ranking.retrieval_sources,
			ranking_override.as_ref().and_then(|override_| override_.retrieval_sources.as_ref()),
		)?;
		let policy_snapshot = build_policy_snapshot(
			&self.cfg,
			&blend_policy,
			&diversity_policy,
			&retrieval_sources_policy,
			ranking_override.as_ref(),
		);
		let policy_hash = hash_policy_snapshot(&policy_snapshot)?;
		let policy_id = format!("ranking_v2:{}", &policy_hash[..12.min(policy_hash.len())]);
		let mut scored: Vec<ScoredChunk> = Vec::new();

		if !snippet_items.is_empty() {
			let mut cached_scores: Option<Vec<f32>> = None;
			let mut cache_key: Option<String> = None;
			let mut cache_candidates: Vec<RerankCacheCandidate> = Vec::new();

			if cache_cfg.enabled {
				let candidates: Vec<RerankCacheCandidate> = snippet_items
					.iter()
					.map(|item| RerankCacheCandidate {
						chunk_id: item.chunk.chunk_id,
						updated_at: item.note.updated_at,
					})
					.collect();
				let signature: Vec<(Uuid, OffsetDateTime)> = candidates
					.iter()
					.map(|candidate| (candidate.chunk_id, candidate.updated_at))
					.collect();

				match build_rerank_cache_key(
					query,
					self.cfg.providers.rerank.provider_id.as_str(),
					self.cfg.providers.rerank.model.as_str(),
					&signature,
				) {
					Ok(key) => {
						cache_key = Some(key.clone());
						cache_candidates = candidates;

						match fetch_cache_payload(&self.db.pool, CacheKind::Rerank, &key, now).await
						{
							Ok(Some(payload)) => {
								let decoded: RerankCachePayload =
									match serde_json::from_value(payload.value) {
										Ok(value) => value,
										Err(err) => {
											tracing::warn!(
												error = %err,
												cache_kind = CacheKind::Rerank.as_str(),
												cache_key_prefix = cache_key_prefix(&key),
												"Cache payload decode failed."
											);

											RerankCachePayload { items: Vec::new() }
										},
									};

								if let Some(scores) =
									build_cached_scores(&decoded, &cache_candidates)
								{
									tracing::info!(
										cache_kind = CacheKind::Rerank.as_str(),
										cache_key_prefix = cache_key_prefix(&key),
										hit = true,
										payload_size = payload.size_bytes,
										ttl_days = cache_cfg.rerank_ttl_days,
										"Cache hit."
									);

									cached_scores = Some(scores);
								} else {
									tracing::warn!(
										cache_kind = CacheKind::Rerank.as_str(),
										cache_key_prefix = cache_key_prefix(&key),
										hit = false,
										payload_size = payload.size_bytes,
										ttl_days = cache_cfg.rerank_ttl_days,
										"Cache payload did not match candidates."
									);
								}
							},
							Ok(None) => {
								tracing::info!(
									cache_kind = CacheKind::Rerank.as_str(),
									cache_key_prefix = cache_key_prefix(&key),
									hit = false,
									payload_size = 0_u64,
									ttl_days = cache_cfg.rerank_ttl_days,
									"Cache miss."
								);
							},
							Err(err) => {
								tracing::warn!(
									error = %err,
									cache_kind = CacheKind::Rerank.as_str(),
									cache_key_prefix = cache_key_prefix(&key),
									"Cache read failed."
								);
							},
						}
					},
					Err(err) => {
						tracing::warn!(
							error = %err,
							cache_kind = CacheKind::Rerank.as_str(),
							"Cache key build failed."
						);
					},
				}
			}

			let scores = if let Some(scores) = cached_scores {
				scores
			} else {
				let docs: Vec<String> =
					snippet_items.iter().map(|item| item.snippet.clone()).collect();
				let scores =
					self.providers.rerank.rerank(&self.cfg.providers.rerank, query, &docs).await?;

				if scores.len() != snippet_items.len() {
					return Err(Error::Provider {
						message: "Rerank provider returned mismatched score count.".to_string(),
					});
				}
				if cache_cfg.enabled
					&& let Some(key) = cache_key.as_ref()
					&& !cache_candidates.is_empty()
				{
					let payload = RerankCachePayload {
						items: cache_candidates
							.iter()
							.zip(scores.iter())
							.map(|(candidate, score)| RerankCacheItem {
								chunk_id: candidate.chunk_id,
								updated_at: candidate.updated_at,
								score: *score,
							})
							.collect(),
					};

					match serde_json::to_value(&payload) {
						Ok(payload_json) => {
							let stored_at = OffsetDateTime::now_utc();
							let expires_at = stored_at + Duration::days(cache_cfg.rerank_ttl_days);

							match store_cache_payload(
								&self.db.pool,
								CacheKind::Rerank,
								key,
								payload_json,
								stored_at,
								expires_at,
								cache_cfg.max_payload_bytes,
							)
							.await
							{
								Ok(Some(payload_size)) => {
									tracing::info!(
										cache_kind = CacheKind::Rerank.as_str(),
										cache_key_prefix = cache_key_prefix(key),
										hit = false,
										payload_size,
										ttl_days = cache_cfg.rerank_ttl_days,
										"Cache stored."
									);
								},
								Ok(None) => {
									tracing::warn!(
										cache_kind = CacheKind::Rerank.as_str(),
										cache_key_prefix = cache_key_prefix(key),
										hit = false,
										payload_size = 0_u64,
										ttl_days = cache_cfg.rerank_ttl_days,
										"Cache payload skipped due to size."
									);
								},
								Err(err) => {
									tracing::warn!(
										error = %err,
										cache_kind = CacheKind::Rerank.as_str(),
										cache_key_prefix = cache_key_prefix(key),
										"Cache write failed."
									);
								},
							}
						},
						Err(err) => {
							tracing::warn!(
								error = %err,
								cache_kind = CacheKind::Rerank.as_str(),
								cache_key_prefix = cache_key_prefix(key),
								"Cache payload encode failed."
							);
						},
					}
				}

				scores
			};

			scored = Vec::with_capacity(snippet_items.len());

			let rerank_ranks = build_rerank_ranks(&snippet_items, &scores);
			let total_rerank = u32::try_from(scores.len()).unwrap_or(1).max(1);
			let total_retrieval = u32::try_from(candidate_count).unwrap_or(1).max(1);

			for ((item, rerank_score), rerank_rank) in
				snippet_items.into_iter().zip(scores.into_iter()).zip(rerank_ranks.into_iter())
			{
				let importance = item.note.importance;
				let retrieval_rank = item.retrieval_rank;
				let age_days = (now - item.note.updated_at).as_seconds_f32() / 86_400.0;
				let decay = if self.cfg.ranking.recency_tau_days > 0.0 {
					(-age_days / self.cfg.ranking.recency_tau_days).exp()
				} else {
					1.0
				};
				let base = (1.0 + 0.6 * importance) * decay;
				let tie_breaker_score = self.cfg.ranking.tie_breaker_weight * base;
				let scope_context_boost = scope_context_boost_by_scope
					.get(item.note.scope.as_str())
					.copied()
					.unwrap_or(0.0);
				let rerank_norm = match blend_policy.rerank_normalization {
					NormalizationKind::Rank => rank_normalize(rerank_rank, total_rerank),
				};
				let retrieval_norm = match blend_policy.retrieval_normalization {
					NormalizationKind::Rank => rank_normalize(retrieval_rank, total_retrieval),
				};
				let blend_retrieval_weight = if blend_policy.enabled {
					retrieval_weight_for_rank(retrieval_rank, &blend_policy.segments)
				} else {
					0.0
				};
				let retrieval_term = blend_retrieval_weight * retrieval_norm;
				let rerank_term = (1.0 - blend_retrieval_weight) * rerank_norm;
				let det_terms = compute_deterministic_ranking_terms(
					&self.cfg,
					&det_query_tokens,
					item.snippet.as_str(),
					item.note.hit_count,
					item.note.last_hit_at,
					age_days,
					now,
				);
				let final_score = retrieval_term
					+ rerank_term + tie_breaker_score
					+ scope_context_boost
					+ det_terms.lexical_bonus
					+ det_terms.hit_boost
					+ det_terms.decay_penalty;

				scored.push(ScoredChunk {
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
				});
			}
		}

		let mut best_by_note: HashMap<Uuid, ScoredChunk> = HashMap::new();
		let mut trace_candidates = if self.cfg.search.explain.capture_candidates {
			let candidate_expires_at =
				now + Duration::days(self.cfg.search.explain.candidate_retention_days);

			scored
				.iter()
				.map(|scored_chunk| {
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
						expires_at: candidate_expires_at,
					}
				})
				.collect::<Vec<_>>()
		} else {
			Vec::new()
		};

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

		results.sort_by(|a, b| {
			let ord = cmp_f32_desc(a.final_score, b.final_score);

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
		});
		let note_vectors = if diversity_policy.enabled {
			fetch_note_vectors_for_diversity(&self.db.pool, &results).await?
		} else {
			HashMap::new()
		};
		let (selected_results, diversity_decisions) =
			select_diverse_results(results, top_k, &diversity_policy, &note_vectors);
		attach_diversity_decisions_to_trace_candidates(&mut trace_candidates, &diversity_decisions);

		if record_hits_enabled && !selected_results.is_empty() {
			let mut tx = self.db.pool.begin().await?;

			record_hits(&mut *tx, query, &selected_results, now).await?;
			tx.commit().await?;
		}

		let trace_context = TraceContext {
			trace_id,
			tenant_id,
			project_id,
			agent_id,
			read_profile,
			query,
			expansion_mode,
			expanded_queries,
			allowed_scopes,
			candidate_count,
			top_k,
		};
		let config_snapshot = build_config_snapshot(
			&self.cfg,
			&blend_policy,
			&diversity_policy,
			&retrieval_sources_policy,
			ranking_override.as_ref(),
			policy_id.as_str(),
			&policy_snapshot,
		);
		let mut items = Vec::with_capacity(selected_results.len());
		let mut trace_builder = SearchTraceBuilder::new(
			trace_context,
			config_snapshot,
			self.cfg.search.explain.retention_days,
			now,
		);

		for candidate in trace_candidates {
			trace_builder.push_candidate(candidate);
		}

		for (idx, scored_chunk) in selected_results.into_iter().enumerate() {
			let rank = idx as u32 + 1;
			let (matched_terms, matched_fields) = match_terms_in_text(
				&query_tokens,
				&scored_chunk.item.snippet,
				scored_chunk.item.note.key.as_deref(),
				MAX_MATCHED_TERMS,
			);
			let matched_fields = merge_matched_fields(
				matched_fields,
				structured_matches.get(&scored_chunk.item.note.note_id),
			);
			let trace_terms =
				ranking_explain_v2::build_trace_terms_v2(ranking_explain_v2::TraceTermsArgs {
					cfg: &self.cfg,
					blend_enabled: blend_policy.enabled,
					retrieval_normalization: blend_policy.retrieval_normalization.as_str(),
					rerank_normalization: blend_policy.rerank_normalization.as_str(),
					blend_retrieval_weight: scored_chunk.blend_retrieval_weight,
					retrieval_rank: scored_chunk.item.retrieval_rank,
					retrieval_norm: scored_chunk.retrieval_norm,
					retrieval_term: scored_chunk.retrieval_term,
					rerank_score: scored_chunk.rerank_score,
					rerank_rank: scored_chunk.rerank_rank,
					rerank_norm: scored_chunk.rerank_norm,
					rerank_term: scored_chunk.rerank_term,
					tie_breaker_score: scored_chunk.tie_breaker_score,
					importance: scored_chunk.importance,
					age_days: scored_chunk.age_days,
					scope: scored_chunk.item.note.scope.as_str(),
					scope_context_boost: scored_chunk.scope_context_boost,
					deterministic_lexical_overlap_ratio: scored_chunk
						.deterministic_lexical_overlap_ratio,
					deterministic_lexical_bonus: scored_chunk.deterministic_lexical_bonus,
					deterministic_hit_count: scored_chunk.deterministic_hit_count,
					deterministic_last_hit_age_days: scored_chunk.deterministic_last_hit_age_days,
					deterministic_hit_boost: scored_chunk.deterministic_hit_boost,
					deterministic_decay_penalty: scored_chunk.deterministic_decay_penalty,
				});
			let response_terms = ranking_explain_v2::strip_term_inputs(&trace_terms);
			let response_explain = SearchExplain {
				r#match: SearchMatchExplain {
					matched_terms: matched_terms.clone(),
					matched_fields: matched_fields.clone(),
				},
				ranking: SearchRankingExplain {
					schema: ranking_explain_v2::SEARCH_RANKING_EXPLAIN_SCHEMA_V2.to_string(),
					policy_id: policy_id.clone(),
					final_score: scored_chunk.final_score,
					terms: response_terms,
				},
				diversity: if diversity_policy.enabled {
					diversity_decisions
						.get(&scored_chunk.item.note.note_id)
						.map(build_diversity_explain)
				} else {
					None
				},
			};
			let trace_explain = SearchExplain {
				r#match: SearchMatchExplain { matched_terms, matched_fields },
				ranking: SearchRankingExplain {
					schema: ranking_explain_v2::SEARCH_RANKING_EXPLAIN_SCHEMA_V2.to_string(),
					policy_id: policy_id.clone(),
					final_score: scored_chunk.final_score,
					terms: trace_terms,
				},
				diversity: if diversity_policy.enabled {
					diversity_decisions
						.get(&scored_chunk.item.note.note_id)
						.map(build_diversity_explain)
				} else {
					None
				},
			};
			let result_handle = Uuid::new_v4();
			let note = &scored_chunk.item.note;
			let chunk = &scored_chunk.item.chunk;

			items.push(SearchItem {
				result_handle,
				note_id: note.note_id,
				chunk_id: chunk.chunk_id,
				chunk_index: chunk.chunk_index,
				start_offset: chunk.start_offset,
				end_offset: chunk.end_offset,
				snippet: scored_chunk.item.snippet.clone(),
				r#type: note.note_type.clone(),
				key: note.key.clone(),
				scope: note.scope.clone(),
				importance: note.importance,
				confidence: note.confidence,
				updated_at: note.updated_at,
				expires_at: note.expires_at,
				final_score: scored_chunk.final_score,
				source_ref: note.source_ref.clone(),
				explain: response_explain.clone(),
			});
			trace_builder.push_item(TraceItemRecord {
				item_id: result_handle,
				note_id: note.note_id,
				chunk_id: Some(chunk.chunk_id),
				rank,
				final_score: scored_chunk.final_score,
				explain: trace_explain,
			});
		}

		let trace_payload = trace_builder.build();

		match self.cfg.search.explain.write_mode.trim().to_ascii_lowercase().as_str() {
			"inline" => {
				let mut tx = self.db.pool.begin().await?;

				persist_trace_inline(&mut tx, trace_payload).await?;
				tx.commit().await?;
			},
			_ =>
				if let Err(err) = enqueue_trace(&self.db.pool, trace_payload).await {
					tracing::error!(
						error = %err,
						trace_id = %trace_id,
						"Failed to enqueue search trace."
					);
				},
		}

		Ok(SearchResponse { trace_id, items })
	}
}

pub fn ranking_policy_id(
	cfg: &Config,
	ranking_override: Option<&RankingRequestOverride>,
) -> Result<String> {
	let blend_policy = resolve_blend_policy(
		&cfg.ranking.blend,
		ranking_override.and_then(|value| value.blend.as_ref()),
	)?;
	let diversity_policy = resolve_diversity_policy(
		&cfg.ranking.diversity,
		ranking_override.and_then(|value| value.diversity.as_ref()),
	)?;
	let retrieval_sources_policy = resolve_retrieval_sources_policy(
		&cfg.ranking.retrieval_sources,
		ranking_override.and_then(|value| value.retrieval_sources.as_ref()),
	)?;
	let snapshot = build_policy_snapshot(
		cfg,
		&blend_policy,
		&diversity_policy,
		&retrieval_sources_policy,
		ranking_override,
	);
	let hash = hash_policy_snapshot(&snapshot)?;
	let prefix = &hash[..12.min(hash.len())];

	Ok(format!("ranking_v2:{prefix}"))
}

pub fn replay_ranking_from_candidates(
	cfg: &Config,
	trace: &TraceReplayContext,
	ranking_override: Option<&RankingRequestOverride>,
	candidates: &[TraceReplayCandidate],
	top_k: u32,
) -> Result<Vec<TraceReplayItem>> {
	#[derive(Clone, Debug)]
	struct ScoredReplay {
		note_id: Uuid,
		chunk_id: Uuid,
		retrieval_rank: u32,
		final_score: f32,
		rerank_score: f32,
		rerank_rank: u32,
		rerank_norm: f32,
		retrieval_norm: f32,
		blend_retrieval_weight: f32,
		retrieval_term: f32,
		rerank_term: f32,
		tie_breaker_score: f32,
		scope_context_boost: f32,
		age_days: f32,
		importance: f32,
		note_scope: String,
		deterministic_lexical_overlap_ratio: f32,
		deterministic_lexical_bonus: f32,
		deterministic_hit_count: i64,
		deterministic_last_hit_age_days: Option<f32>,
		deterministic_hit_boost: f32,
		deterministic_decay_penalty: f32,
	}

	let query_tokens = tokenize_query(trace.query.as_str(), MAX_MATCHED_TERMS);
	let scope_context_boost_by_scope =
		build_scope_context_boost_by_scope(&query_tokens, cfg.context.as_ref());
	let det_query_tokens = if cfg.ranking.deterministic.enabled
		&& cfg.ranking.deterministic.lexical.enabled
		&& cfg.ranking.deterministic.lexical.max_query_terms > 0
	{
		tokenize_query(
			trace.query.as_str(),
			cfg.ranking.deterministic.lexical.max_query_terms as usize,
		)
	} else {
		Vec::new()
	};
	let blend_policy = resolve_blend_policy(
		&cfg.ranking.blend,
		ranking_override.and_then(|override_| override_.blend.as_ref()),
	)?;
	let diversity_policy = resolve_diversity_policy(
		&cfg.ranking.diversity,
		ranking_override.and_then(|override_| override_.diversity.as_ref()),
	)?;
	let retrieval_sources_policy = resolve_retrieval_sources_policy(
		&cfg.ranking.retrieval_sources,
		ranking_override.and_then(|override_| override_.retrieval_sources.as_ref()),
	)?;
	let policy_snapshot = build_policy_snapshot(
		cfg,
		&blend_policy,
		&diversity_policy,
		&retrieval_sources_policy,
		ranking_override,
	);
	let policy_hash = hash_policy_snapshot(&policy_snapshot)?;
	let policy_id = format!("ranking_v2:{}", &policy_hash[..12.min(policy_hash.len())]);
	let now = trace.created_at;
	let total_rerank = u32::try_from(candidates.len()).unwrap_or(1).max(1);
	let total_retrieval = trace.candidate_count.max(1);
	let rerank_ranks = build_rerank_ranks_for_replay(candidates);
	let replay_diversity_decisions = extract_replay_diversity_decisions(candidates);
	let mut best_by_note: BTreeMap<Uuid, ScoredReplay> = BTreeMap::new();

	for (candidate, rerank_rank) in candidates.iter().zip(rerank_ranks) {
		let importance = candidate.note_importance;
		let retrieval_rank = candidate.retrieval_rank;
		let age_days = (now - candidate.note_updated_at).as_seconds_f32() / 86_400.0;
		let decay = if cfg.ranking.recency_tau_days > 0.0 {
			(-age_days / cfg.ranking.recency_tau_days).exp()
		} else {
			1.0
		};
		let base = (1.0 + 0.6 * importance) * decay;
		let tie_breaker_score = cfg.ranking.tie_breaker_weight * base;
		let scope_context_boost =
			scope_context_boost_by_scope.get(candidate.note_scope.as_str()).copied().unwrap_or(0.0);
		let rerank_norm = match blend_policy.rerank_normalization {
			NormalizationKind::Rank => rank_normalize(rerank_rank, total_rerank),
		};
		let retrieval_norm = match blend_policy.retrieval_normalization {
			NormalizationKind::Rank => rank_normalize(retrieval_rank, total_retrieval),
		};
		let blend_retrieval_weight = if blend_policy.enabled {
			retrieval_weight_for_rank(retrieval_rank, &blend_policy.segments)
		} else {
			0.0
		};
		let retrieval_term = blend_retrieval_weight * retrieval_norm;
		let rerank_term = (1.0 - blend_retrieval_weight) * rerank_norm;
		let det_terms = compute_deterministic_ranking_terms(
			cfg,
			&det_query_tokens,
			candidate.snippet.as_str(),
			candidate.note_hit_count,
			candidate.note_last_hit_at,
			age_days,
			now,
		);
		let final_score = retrieval_term
			+ rerank_term
			+ tie_breaker_score
			+ scope_context_boost
			+ det_terms.lexical_bonus
			+ det_terms.hit_boost
			+ det_terms.decay_penalty;
		let scored = ScoredReplay {
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
		};
		let replace = match best_by_note.get(&candidate.note_id) {
			None => true,
			Some(existing) => {
				let ord = cmp_f32_desc(scored.final_score, existing.final_score);
				if ord != Ordering::Equal {
					ord == Ordering::Less
				} else {
					scored.retrieval_rank < existing.retrieval_rank
				}
			},
		};

		if replace {
			best_by_note.insert(candidate.note_id, scored);
		}
	}

	let mut results: Vec<ScoredReplay> = best_by_note.into_values().collect();

	results.sort_by(|a, b| {
		let ord = cmp_f32_desc(a.final_score, b.final_score);

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
	});

	if diversity_policy.enabled && !replay_diversity_decisions.is_empty() {
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

	let mut out = Vec::with_capacity(results.len());

	for scored in results {
		let terms = ranking_explain_v2::build_trace_terms_v2(ranking_explain_v2::TraceTermsArgs {
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
				schema: ranking_explain_v2::SEARCH_RANKING_EXPLAIN_SCHEMA_V2.to_string(),
				policy_id: policy_id.clone(),
				final_score: scored.final_score,
				terms,
			},
			diversity: if diversity_policy.enabled {
				replay_diversity_decisions.get(&scored.note_id).map(build_diversity_explain)
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

	Ok(out)
}

fn resolve_expansion_mode(cfg: &Config) -> ExpansionMode {
	match cfg.search.expansion.mode.as_str() {
		"off" => ExpansionMode::Off,
		"always" => ExpansionMode::Always,
		"dynamic" => ExpansionMode::Dynamic,
		_ => ExpansionMode::Off,
	}
}

fn should_expand_dynamic(
	candidate_count: usize,
	top_score: f32,
	cfg: &elf_config::SearchDynamic,
) -> bool {
	candidate_count < cfg.min_candidates as usize || top_score < cfg.min_top_score
}

fn normalize_queries(
	queries: Vec<String>,
	original: &str,
	include_original: bool,
	max_queries: u32,
) -> Vec<String> {
	let mut out = Vec::new();
	let mut seen = HashSet::new();

	if include_original {
		push_query(&mut out, &mut seen, original);
	}

	for query in queries {
		if out.len() >= max_queries as usize {
			break;
		}

		push_query(&mut out, &mut seen, &query);
	}

	out.truncate(max_queries as usize);

	out
}

fn push_query(out: &mut Vec<String>, seen: &mut HashSet<String>, value: &str) {
	let trimmed = value.trim();

	if trimmed.is_empty() || cjk::contains_cjk(trimmed) {
		return;
	}

	let key = trimmed.to_lowercase();

	if seen.insert(key) {
		out.push(trimmed.to_string());
	}
}

fn build_expansion_messages(
	query: &str,
	max_queries: u32,
	include_original: bool,
) -> Vec<serde_json::Value> {
	let schema = serde_json::json!({
		"queries": ["string"]
	});
	let schema_text = serde_json::to_string_pretty(&schema)
		.unwrap_or_else(|_| "{\"queries\": [\"string\"]}".to_string());
	let system_prompt = "You are a query expansion engine for a memory retrieval system. \
Output must be valid JSON only and must match the provided schema exactly. \
Generate short English-only query variations that preserve the original intent. \
Do not include any CJK characters. Do not add explanations or extra fields.";
	let user_prompt = format!(
		"Return JSON matching this exact schema:\n{schema}\nConstraints:\n- MAX_QUERIES = {max}\n- INCLUDE_ORIGINAL = {include}\nOriginal query:\n{query}",
		schema = schema_text,
		max = max_queries,
		include = include_original,
		query = query
	);

	vec![
		serde_json::json!({ "role": "system", "content": system_prompt }),
		serde_json::json!({ "role": "user", "content": user_prompt }),
	]
}

fn collect_chunk_candidates(
	points: &[ScoredPoint],
	max_candidates: u32,
	candidate_k: u32,
) -> Vec<ChunkCandidate> {
	let limit = if max_candidates == 0 || max_candidates >= candidate_k {
		points.len()
	} else {
		max_candidates as usize
	};

	let mut out = Vec::new();
	let mut seen = HashSet::new();

	for (idx, point) in points.iter().take(limit).enumerate() {
		let chunk_id = point
			.id
			.as_ref()
			.and_then(point_id_to_uuid)
			.or_else(|| payload_uuid(&point.payload, "chunk_id"));
		let Some(chunk_id) = chunk_id else {
			tracing::warn!("Chunk candidate missing chunk_id.");
			continue;
		};

		if !seen.insert(chunk_id) {
			continue;
		}

		let Some(note_id) = payload_uuid(&point.payload, "note_id") else {
			tracing::warn!(chunk_id = %chunk_id, "Chunk candidate missing note_id.");
			continue;
		};
		let Some(chunk_index) = payload_i32(&point.payload, "chunk_index") else {
			tracing::warn!(chunk_id = %chunk_id, "Chunk candidate missing chunk_index.");
			continue;
		};
		let updated_at = payload_rfc3339(&point.payload, "updated_at");
		let embedding_version = payload_string(&point.payload, "embedding_version");

		out.push(ChunkCandidate {
			chunk_id,
			note_id,
			chunk_index,
			retrieval_rank: idx as u32 + 1,
			updated_at,
			embedding_version,
		});
	}

	out
}

fn retrieval_source_weight(
	policy: &ResolvedRetrievalSourcesPolicy,
	source: RetrievalSourceKind,
) -> f32 {
	match source {
		RetrievalSourceKind::Fusion => policy.fusion_weight,
		RetrievalSourceKind::StructuredField => policy.structured_field_weight,
	}
}

fn retrieval_source_priority(
	policy: &ResolvedRetrievalSourcesPolicy,
	source: RetrievalSourceKind,
) -> u32 {
	match source {
		RetrievalSourceKind::StructuredField => policy.structured_field_priority,
		RetrievalSourceKind::Fusion => policy.fusion_priority,
	}
}

fn retrieval_source_kind_order(source: RetrievalSourceKind) -> u8 {
	match source {
		RetrievalSourceKind::StructuredField => 0,
		RetrievalSourceKind::Fusion => 1,
	}
}

fn merge_retrieval_candidates(
	sources: Vec<RetrievalSourceCandidates>,
	policy: &ResolvedRetrievalSourcesPolicy,
	candidate_k: u32,
) -> Vec<ChunkCandidate> {
	if candidate_k == 0 {
		return Vec::new();
	}

	#[derive(Debug)]
	struct MergedRetrievalCandidate {
		candidate: ChunkCandidate,
		source_ranks: HashMap<RetrievalSourceKind, u32>,
		combined_score: f32,
	}

	let mut by_chunk: HashMap<Uuid, MergedRetrievalCandidate> = HashMap::new();
	let mut source_totals: HashMap<RetrievalSourceKind, u32> = HashMap::new();

	for source in sources {
		let mut seen_for_source = HashSet::new();

		for candidate in &source.candidates {
			if seen_for_source.insert(candidate.chunk_id) {
				*source_totals.entry(source.source).or_insert(0) += 1;
			}
		}

		for candidate in source.candidates {
			let chunk_id = candidate.chunk_id;
			let rank = candidate.retrieval_rank;

			match by_chunk.get_mut(&chunk_id) {
				Some(existing) => {
					let entry = existing.source_ranks.entry(source.source).or_insert(rank);

					*entry = (*entry).min(rank);
				},
				None => {
					let mut source_ranks = HashMap::new();

					source_ranks.insert(source.source, rank);
					by_chunk.insert(
						chunk_id,
						MergedRetrievalCandidate { candidate, source_ranks, combined_score: 0.0 },
					);
				},
			}
		}
	}

	if by_chunk.is_empty() {
		return Vec::new();
	}

	for total in source_totals.values_mut() {
		*total = (*total).max(1);
	}

	let mut source_order: Vec<RetrievalSourceKind> = source_totals.keys().copied().collect();

	source_order.sort_by(|left, right| {
		retrieval_source_priority(policy, *left)
			.cmp(&retrieval_source_priority(policy, *right))
			.then_with(|| {
				retrieval_source_kind_order(*left).cmp(&retrieval_source_kind_order(*right))
			})
	});

	let mut merged: Vec<MergedRetrievalCandidate> = by_chunk.into_values().collect();

	for candidate in &mut merged {
		let mut combined_score = 0.0_f32;

		for (source, rank) in &candidate.source_ranks {
			let total = source_totals.get(source).copied().unwrap_or(1);

			combined_score +=
				retrieval_source_weight(policy, *source) * rank_normalize(*rank, total);
		}
		candidate.combined_score = combined_score;
	}

	merged.sort_by(|left, right| {
		cmp_f32_desc(left.combined_score, right.combined_score)
			.then_with(|| right.source_ranks.len().cmp(&left.source_ranks.len()))
			.then_with(|| {
				for source in &source_order {
					let lhs = left.source_ranks.get(source).copied();
					let rhs = right.source_ranks.get(source).copied();
					let ord = rank_asc(lhs, rhs);

					if ord != Ordering::Equal {
						return ord;
					}
				}

				Ordering::Equal
			})
			.then_with(|| left.candidate.chunk_id.cmp(&right.candidate.chunk_id))
	});

	let mut out = Vec::new();

	for (idx, mut candidate) in merged.into_iter().take(candidate_k as usize).enumerate() {
		candidate.candidate.retrieval_rank = idx as u32 + 1;

		out.push(candidate.candidate);
	}

	out
}

fn rank_asc(left: Option<u32>, right: Option<u32>) -> Ordering {
	let lhs = left.unwrap_or(u32::MAX);
	let rhs = right.unwrap_or(u32::MAX);

	lhs.cmp(&rhs)
}

fn candidate_matches_note(note_meta: &HashMap<Uuid, NoteMeta>, candidate: &ChunkCandidate) -> bool {
	let Some(note) = note_meta.get(&candidate.note_id) else { return false };

	if let Some(version) = candidate.embedding_version.as_deref()
		&& version != note.embedding_version.as_str()
	{
		return false;
	}
	if let Some(ts) = candidate.updated_at
		&& ts != note.updated_at
	{
		return false;
	}

	true
}

fn collect_neighbor_pairs(candidates: &[ChunkCandidate]) -> Vec<(Uuid, i32)> {
	let mut seen = HashSet::new();
	let mut out = Vec::new();

	for candidate in candidates {
		let mut indices = Vec::with_capacity(3);

		indices.push(candidate.chunk_index);

		if let Some(prev) = candidate.chunk_index.checked_sub(1) {
			indices.push(prev);
		}
		if let Some(next) = candidate.chunk_index.checked_add(1) {
			indices.push(next);
		}

		for idx in indices {
			let key = (candidate.note_id, idx);

			if seen.insert(key) {
				out.push(key);
			}
		}
	}

	out
}

fn stitch_snippet(
	note_id: Uuid,
	chunk_index: i32,
	chunks: &HashMap<(Uuid, i32), ChunkRow>,
) -> String {
	let indices = [chunk_index.checked_sub(1), Some(chunk_index), chunk_index.checked_add(1)];

	let mut out = String::new();

	for index in indices.into_iter().flatten() {
		if let Some(chunk) = chunks.get(&(note_id, index)) {
			out.push_str(chunk.text.as_str());
		}
	}

	out.trim().to_string()
}

fn expansion_mode_label(mode: ExpansionMode) -> &'static str {
	match mode {
		ExpansionMode::Off => "off",
		ExpansionMode::Always => "always",
		ExpansionMode::Dynamic => "dynamic",
	}
}

fn build_dense_embedding_input(query: &str, project_context_description: Option<&str>) -> String {
	let Some(description) = project_context_description else { return query.to_string() };
	let trimmed = description.trim();

	if trimmed.is_empty() {
		return query.to_string();
	}

	format!("{query}\n\nProject context:\n{trimmed}")
}

fn build_scope_context_boost_by_scope<'a>(
	tokens: &[String],
	context: Option<&'a elf_config::Context>,
) -> HashMap<&'a str, f32> {
	let Some(context) = context else { return HashMap::new() };
	let Some(weight) = context.scope_boost_weight else { return HashMap::new() };

	if weight <= 0.0 || tokens.is_empty() {
		return HashMap::new();
	}

	let Some(descriptions) = context.scope_descriptions.as_ref() else { return HashMap::new() };
	let mut out = HashMap::new();

	for (scope, description) in descriptions {
		let boost = scope_description_boost(tokens, description, weight);

		if boost > 0.0 {
			out.insert(scope.as_str(), boost);
		}
	}

	out
}

fn scope_description_boost(tokens: &[String], description: &str, weight: f32) -> f32 {
	if weight <= 0.0 || tokens.is_empty() {
		return 0.0;
	}

	let trimmed = description.trim();

	if trimmed.is_empty() || cjk::contains_cjk(trimmed) {
		return 0.0;
	}

	let mut normalized = String::with_capacity(trimmed.len());

	for ch in trimmed.chars() {
		if ch.is_ascii_alphanumeric() {
			normalized.push(ch.to_ascii_lowercase());
		} else {
			normalized.push(' ');
		}
	}

	let mut description_tokens = HashSet::new();

	for token in normalized.split_whitespace() {
		if token.len() < 2 {
			continue;
		}

		description_tokens.insert(token);
	}

	if description_tokens.is_empty() {
		return 0.0;
	}

	let mut matched = 0usize;

	for token in tokens {
		if description_tokens.contains(token.as_str()) {
			matched += 1;
		}
	}

	if matched == 0 {
		return 0.0;
	}

	weight * (matched as f32 / tokens.len() as f32)
}

fn tokenize_query(query: &str, max_terms: usize) -> Vec<String> {
	let mut normalized = String::with_capacity(query.len());

	for ch in query.chars() {
		if ch.is_ascii_alphanumeric() {
			normalized.push(ch.to_ascii_lowercase());
		} else {
			normalized.push(' ');
		}
	}

	let mut out = Vec::new();
	let mut seen = HashSet::new();

	for token in normalized.split_whitespace() {
		if token.len() < 2 {
			continue;
		}
		if seen.insert(token) {
			out.push(token.to_string());
		}
		if out.len() >= max_terms {
			break;
		}
	}

	out
}

fn tokenize_text_terms(text: &str, max_terms: usize) -> HashSet<String> {
	if max_terms == 0 {
		return HashSet::new();
	}

	let mut normalized = String::with_capacity(text.len());

	for ch in text.chars() {
		if ch.is_ascii_alphanumeric() {
			normalized.push(ch.to_ascii_lowercase());
		} else {
			normalized.push(' ');
		}
	}

	let mut out = HashSet::new();

	for token in normalized.split_whitespace() {
		if token.len() < 2 {
			continue;
		}

		out.insert(token.to_string());

		if out.len() >= max_terms {
			break;
		}
	}

	out
}

fn lexical_overlap_ratio(query_tokens: &[String], text: &str, max_text_terms: usize) -> f32 {
	if query_tokens.is_empty() {
		return 0.0;
	}

	let text_terms = tokenize_text_terms(text, max_text_terms);

	if text_terms.is_empty() {
		return 0.0;
	}

	let mut matched = 0usize;

	for token in query_tokens {
		if text_terms.contains(token.as_str()) {
			matched += 1;
		}
	}

	matched as f32 / query_tokens.len() as f32
}

fn compute_deterministic_ranking_terms(
	cfg: &Config,
	query_tokens: &[String],
	snippet: &str,
	note_hit_count: i64,
	note_last_hit_at: Option<OffsetDateTime>,
	age_days: f32,
	now: OffsetDateTime,
) -> DeterministicRankingTerms {
	let det = &cfg.ranking.deterministic;

	if !det.enabled {
		return DeterministicRankingTerms::default();
	}

	let mut out = DeterministicRankingTerms::default();

	if det.lexical.enabled && det.lexical.weight > 0.0 && !query_tokens.is_empty() {
		let ratio =
			lexical_overlap_ratio(query_tokens, snippet, det.lexical.max_text_terms as usize);

		out.lexical_overlap_ratio = ratio;

		let min_ratio = det.lexical.min_ratio.clamp(0.0, 1.0);
		let scaled = if ratio >= min_ratio && min_ratio < 1.0 {
			((ratio - min_ratio) / (1.0 - min_ratio)).clamp(0.0, 1.0)
		} else if ratio >= 1.0 && min_ratio >= 1.0 {
			1.0
		} else {
			0.0
		};

		out.lexical_bonus = det.lexical.weight * scaled;
	}

	if det.hits.enabled && det.hits.weight > 0.0 {
		let hit_count = note_hit_count.max(0);

		out.hit_count = hit_count;

		let half = det.hits.half_saturation;
		let hit_saturation = if half > 0.0 && hit_count > 0 {
			let hc = hit_count as f32;

			(hc / (hc + half)).clamp(0.0, 1.0)
		} else {
			0.0
		};

		let last_hit_age_days =
			note_last_hit_at.map(|ts| ((now - ts).as_seconds_f32() / 86_400.0).max(0.0));

		out.last_hit_age_days = last_hit_age_days;

		let tau = det.hits.last_hit_tau_days;
		let recency = if tau > 0.0 {
			match last_hit_age_days {
				Some(days) => (-days / tau).exp(),
				None => 1.0,
			}
		} else {
			1.0
		};

		out.hit_boost = det.hits.weight * hit_saturation * recency;
	}

	if det.decay.enabled && det.decay.weight > 0.0 {
		let age_days = age_days.max(0.0);
		let tau = det.decay.tau_days;
		let staleness = if tau > 0.0 { 1.0 - (-age_days / tau).exp() } else { 0.0 };

		out.decay_penalty = -det.decay.weight * staleness.clamp(0.0, 1.0);
	}

	out
}

fn match_terms_in_text(
	tokens: &[String],
	text: &str,
	key: Option<&str>,
	max_terms: usize,
) -> (Vec<String>, Vec<String>) {
	if tokens.is_empty() {
		return (Vec::new(), Vec::new());
	}

	let text = text.to_lowercase();
	let key = key.map(|value| value.to_lowercase());
	let mut matched_terms = Vec::new();
	let mut matched_fields = HashSet::new();

	for token in tokens {
		let mut matched = false;

		if text.contains(token) {
			matched_fields.insert("text");
			matched = true;
		}

		if let Some(key) = key.as_ref()
			&& key.contains(token)
		{
			matched_fields.insert("key");
			matched = true;
		}

		if matched {
			matched_terms.push(token.clone());
		}
		if matched_terms.len() >= max_terms {
			break;
		}
	}

	let mut fields: Vec<String> =
		matched_fields.into_iter().map(|field| field.to_string()).collect();

	fields.sort();

	(matched_terms, fields)
}

fn merge_matched_fields(mut base: Vec<String>, extra: Option<&Vec<String>>) -> Vec<String> {
	if let Some(extra) = extra {
		for field in extra {
			base.push(field.clone());
		}

		base.sort();
		base.dedup();
	}

	base
}

fn decode_json<T>(value: serde_json::Value, label: &str) -> Result<T>
where
	T: DeserializeOwned,
{
	serde_json::from_value(value)
		.map_err(|err| Error::Storage { message: format!("Invalid {label} value: {err}") })
}

#[derive(Clone, Copy, Debug)]
enum NormalizationKind {
	Rank,
}
impl NormalizationKind {
	fn as_str(self) -> &'static str {
		match self {
			Self::Rank => "rank",
		}
	}
}

#[derive(Clone, Debug)]
struct BlendSegment {
	max_retrieval_rank: u32,
	retrieval_weight: f32,
}

#[derive(Clone, Debug)]
struct ResolvedBlendPolicy {
	enabled: bool,
	rerank_normalization: NormalizationKind,
	retrieval_normalization: NormalizationKind,
	segments: Vec<BlendSegment>,
}

#[derive(Clone, Debug)]
struct ResolvedDiversityPolicy {
	enabled: bool,
	sim_threshold: f32,
	mmr_lambda: f32,
	max_skips: u32,
}

#[derive(Clone, Debug)]
struct ResolvedRetrievalSourcesPolicy {
	fusion_weight: f32,
	structured_field_weight: f32,
	fusion_priority: u32,
	structured_field_priority: u32,
}

fn build_config_snapshot(
	cfg: &Config,
	blend_policy: &ResolvedBlendPolicy,
	diversity_policy: &ResolvedDiversityPolicy,
	retrieval_sources_policy: &ResolvedRetrievalSourcesPolicy,
	ranking_override: Option<&RankingRequestOverride>,
	policy_id: &str,
	policy_snapshot: &serde_json::Value,
) -> serde_json::Value {
	let override_json = ranking_override.and_then(|value| serde_json::to_value(value).ok());
	serde_json::json!({
		"search": {
			"expansion": {
				"mode": cfg.search.expansion.mode.as_str(),
				"max_queries": cfg.search.expansion.max_queries,
				"include_original": cfg.search.expansion.include_original,
			},
			"dynamic": {
				"min_candidates": cfg.search.dynamic.min_candidates,
				"min_top_score": cfg.search.dynamic.min_top_score,
			},
			"prefilter": {
				"max_candidates": cfg.search.prefilter.max_candidates,
			},
			"explain": {
				"retention_days": cfg.search.explain.retention_days,
			},
		},
		"ranking": {
			"policy_id": policy_id,
			"policy_snapshot": policy_snapshot.clone(),
			"recency_tau_days": cfg.ranking.recency_tau_days,
			"tie_breaker_weight": cfg.ranking.tie_breaker_weight,
			"deterministic": {
				"enabled": cfg.ranking.deterministic.enabled,
				"lexical": {
					"enabled": cfg.ranking.deterministic.lexical.enabled,
					"weight": cfg.ranking.deterministic.lexical.weight,
					"min_ratio": cfg.ranking.deterministic.lexical.min_ratio,
					"max_query_terms": cfg.ranking.deterministic.lexical.max_query_terms,
					"max_text_terms": cfg.ranking.deterministic.lexical.max_text_terms,
				},
				"hits": {
					"enabled": cfg.ranking.deterministic.hits.enabled,
					"weight": cfg.ranking.deterministic.hits.weight,
					"half_saturation": cfg.ranking.deterministic.hits.half_saturation,
					"last_hit_tau_days": cfg.ranking.deterministic.hits.last_hit_tau_days,
				},
				"decay": {
					"enabled": cfg.ranking.deterministic.decay.enabled,
					"weight": cfg.ranking.deterministic.decay.weight,
					"tau_days": cfg.ranking.deterministic.decay.tau_days,
				},
			},
				"blend": {
				"enabled": blend_policy.enabled,
				"rerank_normalization": blend_policy.rerank_normalization.as_str(),
				"retrieval_normalization": blend_policy.retrieval_normalization.as_str(),
				"segments": blend_policy
					.segments
					.iter()
					.map(|segment| {
						serde_json::json!({
							"max_retrieval_rank": segment.max_retrieval_rank,
							"retrieval_weight": segment.retrieval_weight,
						})
					})
						.collect::<Vec<_>>(),
				},
				"diversity": {
					"enabled": diversity_policy.enabled,
					"sim_threshold": diversity_policy.sim_threshold,
					"mmr_lambda": diversity_policy.mmr_lambda,
					"max_skips": diversity_policy.max_skips,
				},
				"retrieval_sources": {
					"fusion_weight": retrieval_sources_policy.fusion_weight,
					"structured_field_weight": retrieval_sources_policy.structured_field_weight,
					"fusion_priority": retrieval_sources_policy.fusion_priority,
					"structured_field_priority": retrieval_sources_policy.structured_field_priority,
				},
				"override": override_json,
			},
		"providers": {
			"embedding": {
				"provider_id": cfg.providers.embedding.provider_id.as_str(),
				"model": cfg.providers.embedding.model.as_str(),
				"dimensions": cfg.providers.embedding.dimensions,
			},
			"rerank": {
				"provider_id": cfg.providers.rerank.provider_id.as_str(),
				"model": cfg.providers.rerank.model.as_str(),
			},
		},
		"storage": {
			"qdrant": {
				"vector_dim": cfg.storage.qdrant.vector_dim,
				"collection": cfg.storage.qdrant.collection.as_str(),
			},
		},
		"context": {
			"scope_boost_weight": cfg.context.as_ref().and_then(|ctx| ctx.scope_boost_weight),
			"project_description_count": cfg
				.context
				.as_ref()
				.and_then(|ctx| ctx.project_descriptions.as_ref())
				.map(|descriptions| descriptions.len())
				.unwrap_or(0),
			"scope_description_count": cfg
				.context
				.as_ref()
				.and_then(|ctx| ctx.scope_descriptions.as_ref())
				.map(|descriptions| descriptions.len())
				.unwrap_or(0),
		},
	})
}

fn build_policy_snapshot(
	cfg: &Config,
	blend_policy: &ResolvedBlendPolicy,
	diversity_policy: &ResolvedDiversityPolicy,
	retrieval_sources_policy: &ResolvedRetrievalSourcesPolicy,
	ranking_override: Option<&RankingRequestOverride>,
) -> serde_json::Value {
	let override_json = ranking_override.and_then(|value| serde_json::to_value(value).ok());

	serde_json::json!({
		"ranking": {
			"recency_tau_days": cfg.ranking.recency_tau_days,
			"tie_breaker_weight": cfg.ranking.tie_breaker_weight,
			"deterministic": {
				"enabled": cfg.ranking.deterministic.enabled,
				"lexical": {
					"enabled": cfg.ranking.deterministic.lexical.enabled,
					"weight": cfg.ranking.deterministic.lexical.weight,
					"min_ratio": cfg.ranking.deterministic.lexical.min_ratio,
					"max_query_terms": cfg.ranking.deterministic.lexical.max_query_terms,
					"max_text_terms": cfg.ranking.deterministic.lexical.max_text_terms,
				},
				"hits": {
					"enabled": cfg.ranking.deterministic.hits.enabled,
					"weight": cfg.ranking.deterministic.hits.weight,
					"half_saturation": cfg.ranking.deterministic.hits.half_saturation,
					"last_hit_tau_days": cfg.ranking.deterministic.hits.last_hit_tau_days,
				},
				"decay": {
					"enabled": cfg.ranking.deterministic.decay.enabled,
					"weight": cfg.ranking.deterministic.decay.weight,
					"tau_days": cfg.ranking.deterministic.decay.tau_days,
				},
			},
				"blend": {
				"enabled": blend_policy.enabled,
				"rerank_normalization": blend_policy.rerank_normalization.as_str(),
				"retrieval_normalization": blend_policy.retrieval_normalization.as_str(),
				"segments": blend_policy
					.segments
					.iter()
					.map(|segment| {
						serde_json::json!({
							"max_retrieval_rank": segment.max_retrieval_rank,
							"retrieval_weight": segment.retrieval_weight,
						})
					})
						.collect::<Vec<_>>(),
				},
				"diversity": {
					"enabled": diversity_policy.enabled,
					"sim_threshold": diversity_policy.sim_threshold,
					"mmr_lambda": diversity_policy.mmr_lambda,
					"max_skips": diversity_policy.max_skips,
				},
				"retrieval_sources": {
					"fusion_weight": retrieval_sources_policy.fusion_weight,
					"structured_field_weight": retrieval_sources_policy.structured_field_weight,
					"fusion_priority": retrieval_sources_policy.fusion_priority,
					"structured_field_priority": retrieval_sources_policy.structured_field_priority,
				},
				"override": override_json,
			},
		"context": {
			"scope_boost_weight": cfg.context.as_ref().and_then(|ctx| ctx.scope_boost_weight),
			"project_description_count": cfg
				.context
				.as_ref()
				.and_then(|ctx| ctx.project_descriptions.as_ref())
				.map(|descriptions| descriptions.len())
				.unwrap_or(0),
			"scope_description_count": cfg
				.context
				.as_ref()
				.and_then(|ctx| ctx.scope_descriptions.as_ref())
				.map(|descriptions| descriptions.len())
				.unwrap_or(0),
		},
	})
}

fn hash_policy_snapshot(payload: &serde_json::Value) -> Result<String> {
	let raw = serde_json::to_vec(payload).map_err(|err| Error::Storage {
		message: format!("Failed to encode policy snapshot: {err}"),
	})?;

	Ok(blake3::hash(&raw).to_hex().to_string())
}

fn resolve_blend_policy(
	cfg: &elf_config::RankingBlend,
	override_: Option<&BlendRankingOverride>,
) -> Result<ResolvedBlendPolicy> {
	let enabled = override_.and_then(|value| value.enabled).unwrap_or(cfg.enabled);
	let rerank_norm = override_
		.and_then(|value| value.rerank_normalization.as_deref())
		.unwrap_or(cfg.rerank_normalization.as_str());
	let retrieval_norm = override_
		.and_then(|value| value.retrieval_normalization.as_deref())
		.unwrap_or(cfg.retrieval_normalization.as_str());
	let rerank_normalization =
		parse_normalization_kind(rerank_norm, "ranking.blend.rerank_normalization")?;
	let retrieval_normalization =
		parse_normalization_kind(retrieval_norm, "ranking.blend.retrieval_normalization")?;
	let segments: Vec<BlendSegment> =
		if let Some(override_segments) = override_.and_then(|value| value.segments.as_ref()) {
			override_segments
				.iter()
				.map(|segment| BlendSegment {
					max_retrieval_rank: segment.max_retrieval_rank,
					retrieval_weight: segment.retrieval_weight,
				})
				.collect::<Vec<_>>()
		} else {
			cfg.segments
				.iter()
				.map(|segment| BlendSegment {
					max_retrieval_rank: segment.max_retrieval_rank,
					retrieval_weight: segment.retrieval_weight,
				})
				.collect::<Vec<_>>()
		};

	validate_blend_segments(&segments)?;

	Ok(ResolvedBlendPolicy { enabled, rerank_normalization, retrieval_normalization, segments })
}

fn resolve_diversity_policy(
	cfg: &elf_config::RankingDiversity,
	override_: Option<&DiversityRankingOverride>,
) -> Result<ResolvedDiversityPolicy> {
	let enabled = override_.and_then(|value| value.enabled).unwrap_or(cfg.enabled);
	let sim_threshold =
		override_.and_then(|value| value.sim_threshold).unwrap_or(cfg.sim_threshold);
	let mmr_lambda = override_.and_then(|value| value.mmr_lambda).unwrap_or(cfg.mmr_lambda);
	let max_skips = override_.and_then(|value| value.max_skips).unwrap_or(cfg.max_skips);

	if !sim_threshold.is_finite() {
		return Err(Error::InvalidRequest {
			message: "ranking.diversity.sim_threshold must be a finite number.".to_string(),
		});
	}
	if !(0.0..=1.0).contains(&sim_threshold) {
		return Err(Error::InvalidRequest {
			message: "ranking.diversity.sim_threshold must be in the range 0.0-1.0.".to_string(),
		});
	}
	if !mmr_lambda.is_finite() {
		return Err(Error::InvalidRequest {
			message: "ranking.diversity.mmr_lambda must be a finite number.".to_string(),
		});
	}
	if !(0.0..=1.0).contains(&mmr_lambda) {
		return Err(Error::InvalidRequest {
			message: "ranking.diversity.mmr_lambda must be in the range 0.0-1.0.".to_string(),
		});
	}

	Ok(ResolvedDiversityPolicy { enabled, sim_threshold, mmr_lambda, max_skips })
}

fn resolve_retrieval_sources_policy(
	cfg: &elf_config::RankingRetrievalSources,
	override_: Option<&RetrievalSourcesRankingOverride>,
) -> Result<ResolvedRetrievalSourcesPolicy> {
	let fusion_weight =
		override_.and_then(|value| value.fusion_weight).unwrap_or(cfg.fusion_weight);
	let structured_field_weight = override_
		.and_then(|value| value.structured_field_weight)
		.unwrap_or(cfg.structured_field_weight);
	let fusion_priority =
		override_.and_then(|value| value.fusion_priority).unwrap_or(cfg.fusion_priority);
	let structured_field_priority = override_
		.and_then(|value| value.structured_field_priority)
		.unwrap_or(cfg.structured_field_priority);

	for (path, value) in [
		("ranking.retrieval_sources.fusion_weight", fusion_weight),
		("ranking.retrieval_sources.structured_field_weight", structured_field_weight),
	] {
		if !value.is_finite() {
			return Err(Error::InvalidRequest {
				message: format!("{path} must be a finite number."),
			});
		}
		if value < 0.0 {
			return Err(Error::InvalidRequest {
				message: format!("{path} must be zero or greater."),
			});
		}
	}
	if fusion_weight <= 0.0 && structured_field_weight <= 0.0 {
		return Err(Error::InvalidRequest {
			message: "At least one retrieval source weight must be greater than zero.".to_string(),
		});
	}

	Ok(ResolvedRetrievalSourcesPolicy {
		fusion_weight,
		structured_field_weight,
		fusion_priority,
		structured_field_priority,
	})
}

fn parse_normalization_kind(value: &str, label: &str) -> Result<NormalizationKind> {
	match value.trim().to_ascii_lowercase().as_str() {
		"rank" => Ok(NormalizationKind::Rank),
		other => Err(Error::InvalidRequest {
			message: format!("{label} must be one of: rank. Got {other}."),
		}),
	}
}

fn validate_blend_segments(segments: &[BlendSegment]) -> Result<()> {
	if segments.is_empty() {
		return Err(Error::InvalidRequest {
			message: "ranking.blend.segments must be non-empty.".to_string(),
		});
	}

	let mut last_max = 0_u32;

	for (idx, segment) in segments.iter().enumerate() {
		if segment.max_retrieval_rank == 0 {
			return Err(Error::InvalidRequest {
				message: "ranking.blend.segments.max_retrieval_rank must be greater than zero."
					.to_string(),
			});
		}
		if idx > 0 && segment.max_retrieval_rank <= last_max {
			return Err(Error::InvalidRequest {
				message: "ranking.blend.segments.max_retrieval_rank must be strictly increasing."
					.to_string(),
			});
		}
		if !segment.retrieval_weight.is_finite() {
			return Err(Error::InvalidRequest {
				message: "ranking.blend.segments.retrieval_weight must be a finite number."
					.to_string(),
			});
		}
		if !(0.0..=1.0).contains(&segment.retrieval_weight) {
			return Err(Error::InvalidRequest {
				message: "ranking.blend.segments.retrieval_weight must be in the range 0.0-1.0."
					.to_string(),
			});
		}

		last_max = segment.max_retrieval_rank;
	}

	Ok(())
}

fn retrieval_weight_for_rank(rank: u32, segments: &[BlendSegment]) -> f32 {
	for segment in segments {
		if rank <= segment.max_retrieval_rank {
			return segment.retrieval_weight;
		}
	}

	segments.last().map(|segment| segment.retrieval_weight).unwrap_or(0.5)
}

fn rank_normalize(rank: u32, total: u32) -> f32 {
	if total <= 1 {
		return 1.0;
	}
	if rank == 0 {
		return 0.0;
	}

	let denom = (total - 1) as f32;
	let pos = (rank.saturating_sub(1)) as f32;

	(1.0 - pos / denom).clamp(0.0, 1.0)
}

fn build_diversity_explain(decision: &DiversityDecision) -> SearchDiversityExplain {
	SearchDiversityExplain {
		enabled: true,
		selected_reason: decision.selected_reason.clone(),
		skipped_reason: decision.skipped_reason.clone(),
		nearest_selected_note_id: decision.nearest_selected_note_id,
		similarity: decision.similarity,
		mmr_score: decision.mmr_score,
		missing_embedding: decision.missing_embedding,
	}
}

fn cosine_similarity(lhs: &[f32], rhs: &[f32]) -> Option<f32> {
	if lhs.is_empty() || lhs.len() != rhs.len() {
		return None;
	}

	let mut dot = 0.0_f32;
	let mut lhs_norm = 0.0_f32;
	let mut rhs_norm = 0.0_f32;

	for (l, r) in lhs.iter().zip(rhs.iter()) {
		dot += l * r;
		lhs_norm += l * l;
		rhs_norm += r * r;
	}

	if lhs_norm <= f32::EPSILON || rhs_norm <= f32::EPSILON {
		return None;
	}

	Some((dot / (lhs_norm.sqrt() * rhs_norm.sqrt())).clamp(-1.0, 1.0))
}

fn nearest_selected_similarity(
	note_id: Uuid,
	candidates: &[ScoredChunk],
	selected_indices: &[usize],
	note_vectors: &HashMap<Uuid, Vec<f32>>,
) -> (Option<f32>, Option<Uuid>, bool) {
	let Some(candidate_vec) = note_vectors.get(&note_id) else {
		return (None, None, true);
	};

	let mut best_similarity: Option<f32> = None;
	let mut nearest_note_id: Option<Uuid> = None;

	for selected_idx in selected_indices {
		let selected_note_id = candidates[*selected_idx].item.note.note_id;
		let Some(selected_vec) = note_vectors.get(&selected_note_id) else {
			continue;
		};
		let Some(similarity) = cosine_similarity(candidate_vec, selected_vec) else {
			continue;
		};

		if best_similarity.map(|value| similarity > value).unwrap_or(true) {
			best_similarity = Some(similarity);
			nearest_note_id = Some(selected_note_id);
		}
	}

	(best_similarity, nearest_note_id, false)
}

#[derive(Clone, Copy)]
struct DiversityPick {
	remaining_pos: usize,
	mmr_score: f32,
	nearest_note_id: Option<Uuid>,
	similarity: Option<f32>,
	missing_embedding: bool,
	retrieval_rank: u32,
}

impl DiversityPick {
	fn better_than(self, other: &Self) -> bool {
		self.mmr_score > other.mmr_score
			|| (self.mmr_score == other.mmr_score && self.retrieval_rank < other.retrieval_rank)
	}
}

fn select_diverse_results(
	candidates: Vec<ScoredChunk>,
	top_k: u32,
	policy: &ResolvedDiversityPolicy,
	note_vectors: &HashMap<Uuid, Vec<f32>>,
) -> (Vec<ScoredChunk>, HashMap<Uuid, DiversityDecision>) {
	if candidates.is_empty() || top_k == 0 {
		return (Vec::new(), HashMap::new());
	}

	if !policy.enabled {
		let mut decisions = HashMap::new();
		let mut selected = Vec::new();

		for (idx, candidate) in candidates.into_iter().enumerate() {
			let selected_rank = (idx < top_k as usize).then_some(idx as u32 + 1);
			let is_selected = selected_rank.is_some();
			let note_id = candidate.item.note.note_id;
			let missing_embedding = !note_vectors.contains_key(&note_id);

			decisions.insert(
				note_id,
				DiversityDecision {
					selected: is_selected,
					selected_rank,
					selected_reason: if is_selected {
						"disabled_passthrough".to_string()
					} else {
						"disabled_truncate".to_string()
					},
					skipped_reason: if is_selected {
						None
					} else {
						Some("disabled_truncate".to_string())
					},
					nearest_selected_note_id: None,
					similarity: None,
					mmr_score: None,
					missing_embedding,
				},
			);

			if is_selected {
				selected.push(candidate);
			}
		}

		return (selected, decisions);
	}

	let total = u32::try_from(candidates.len()).unwrap_or(1).max(1);
	let relevance_by_idx: Vec<f32> =
		(0..candidates.len()).map(|idx| rank_normalize(idx as u32 + 1, total)).collect();
	let mut remaining_indices: Vec<usize> = (0..candidates.len()).collect();
	let mut selected_indices: Vec<usize> = Vec::new();
	let mut decisions: HashMap<Uuid, DiversityDecision> = HashMap::new();
	let first_idx = remaining_indices.remove(0);
	let first_note_id = candidates[first_idx].item.note.note_id;
	let first_missing_embedding = !note_vectors.contains_key(&first_note_id);

	selected_indices.push(first_idx);
	decisions.insert(
		first_note_id,
		DiversityDecision {
			selected: true,
			selected_rank: Some(1),
			selected_reason: "top_relevance".to_string(),
			skipped_reason: None,
			nearest_selected_note_id: None,
			similarity: None,
			mmr_score: Some(relevance_by_idx[first_idx]),
			missing_embedding: first_missing_embedding,
		},
	);

	while selected_indices.len() < top_k as usize && !remaining_indices.is_empty() {
		let mut best_non_filtered: Option<DiversityPick> = None;
		let mut best_filtered: Option<DiversityPick> = None;
		let mut best_any: Option<DiversityPick> = None;
		let mut filtered_count = 0_u32;

		for (remaining_pos, candidate_idx) in remaining_indices.iter().copied().enumerate() {
			let note_id = candidates[candidate_idx].item.note.note_id;
			let (similarity, nearest_note_id, missing_embedding) =
				nearest_selected_similarity(note_id, &candidates, &selected_indices, note_vectors);
			let redundancy = similarity.unwrap_or(0.0);
			let mmr_score = policy.mmr_lambda * relevance_by_idx[candidate_idx]
				- (1.0 - policy.mmr_lambda) * redundancy;
			let high_similarity =
				similarity.map(|value| value > policy.sim_threshold).unwrap_or(false);

			if high_similarity {
				filtered_count += 1;
			}

			let candidate_pick = DiversityPick {
				remaining_pos,
				mmr_score,
				nearest_note_id,
				similarity,
				missing_embedding,
				retrieval_rank: candidates[candidate_idx].item.retrieval_rank,
			};

			if best_any.as_ref().map(|current| candidate_pick.better_than(current)).unwrap_or(true)
			{
				best_any = Some(candidate_pick);
			}
			if high_similarity {
				if best_filtered
					.as_ref()
					.map(|current| candidate_pick.better_than(current))
					.unwrap_or(true)
				{
					best_filtered = Some(candidate_pick);
				}

				continue;
			}
			if best_non_filtered
				.as_ref()
				.map(|current| candidate_pick.better_than(current))
				.unwrap_or(true)
			{
				best_non_filtered = Some(candidate_pick);
			}
		}

		let (selected_pick, selected_reason) = if let Some(best) = best_non_filtered {
			(best, "mmr")
		} else if filtered_count >= policy.max_skips {
			if let Some(best) = best_any {
				(best, "max_skips_backfill")
			} else {
				break;
			}
		} else if let Some(best) = best_filtered {
			(best, "threshold_backfill")
		} else {
			break;
		};

		let picked_idx = remaining_indices.remove(selected_pick.remaining_pos);

		selected_indices.push(picked_idx);

		let selected_note_id = candidates[picked_idx].item.note.note_id;

		decisions.insert(
			selected_note_id,
			DiversityDecision {
				selected: true,
				selected_rank: Some(selected_indices.len() as u32),
				selected_reason: selected_reason.to_string(),
				skipped_reason: None,
				nearest_selected_note_id: selected_pick.nearest_note_id,
				similarity: selected_pick.similarity,
				mmr_score: Some(selected_pick.mmr_score),
				missing_embedding: selected_pick.missing_embedding,
			},
		);
	}

	for candidate_idx in remaining_indices {
		let note_id = candidates[candidate_idx].item.note.note_id;
		let (similarity, nearest_note_id, missing_embedding) =
			nearest_selected_similarity(note_id, &candidates, &selected_indices, note_vectors);
		let skipped_reason =
			if similarity.map(|value| value > policy.sim_threshold).unwrap_or(false) {
				"similarity_threshold"
			} else {
				"lower_mmr"
			};
		let redundancy = similarity.unwrap_or(0.0);
		let mmr_score = policy.mmr_lambda * relevance_by_idx[candidate_idx]
			- (1.0 - policy.mmr_lambda) * redundancy;

		decisions.insert(
			note_id,
			DiversityDecision {
				selected: false,
				selected_rank: None,
				selected_reason: "not_selected".to_string(),
				skipped_reason: Some(skipped_reason.to_string()),
				nearest_selected_note_id: nearest_note_id,
				similarity,
				mmr_score: Some(mmr_score),
				missing_embedding,
			},
		);
	}

	let selected = selected_indices.into_iter().map(|idx| candidates[idx].clone()).collect();

	(selected, decisions)
}

fn attach_diversity_decisions_to_trace_candidates(
	candidates: &mut [TraceCandidateRecord],
	decisions: &HashMap<Uuid, DiversityDecision>,
) {
	for candidate in candidates {
		let Some(decision) = decisions.get(&candidate.note_id) else { continue };
		let mut snapshot = candidate.candidate_snapshot.clone();
		let Some(object) = snapshot.as_object_mut() else { continue };

		object.insert("diversity_selected".to_string(), serde_json::json!(decision.selected));
		object.insert(
			"diversity_selected_rank".to_string(),
			serde_json::json!(decision.selected_rank),
		);
		object.insert(
			"diversity_selected_reason".to_string(),
			serde_json::json!(decision.selected_reason),
		);
		object.insert(
			"diversity_skipped_reason".to_string(),
			serde_json::json!(decision.skipped_reason),
		);
		object.insert(
			"diversity_nearest_selected_note_id".to_string(),
			serde_json::json!(decision.nearest_selected_note_id),
		);
		object.insert("diversity_similarity".to_string(), serde_json::json!(decision.similarity));
		object.insert("diversity_mmr_score".to_string(), serde_json::json!(decision.mmr_score));
		object.insert(
			"diversity_missing_embedding".to_string(),
			serde_json::json!(decision.missing_embedding),
		);

		candidate.candidate_snapshot = snapshot;
	}
}

fn extract_replay_diversity_decisions(
	candidates: &[TraceReplayCandidate],
) -> HashMap<Uuid, DiversityDecision> {
	let mut out: HashMap<Uuid, DiversityDecision> = HashMap::new();

	for candidate in candidates {
		let has_diversity = candidate.diversity_selected.is_some()
			|| candidate.diversity_selected_rank.is_some()
			|| candidate.diversity_selected_reason.is_some()
			|| candidate.diversity_skipped_reason.is_some()
			|| candidate.diversity_nearest_selected_note_id.is_some()
			|| candidate.diversity_similarity.is_some()
			|| candidate.diversity_mmr_score.is_some()
			|| candidate.diversity_missing_embedding.is_some();

		if !has_diversity {
			continue;
		}

		let selected = candidate.diversity_selected.unwrap_or(false);
		let decision = DiversityDecision {
			selected,
			selected_rank: candidate.diversity_selected_rank,
			selected_reason: candidate
				.diversity_selected_reason
				.clone()
				.unwrap_or_else(|| "replay_selected".to_string()),
			skipped_reason: candidate.diversity_skipped_reason.clone(),
			nearest_selected_note_id: candidate.diversity_nearest_selected_note_id,
			similarity: candidate.diversity_similarity,
			mmr_score: candidate.diversity_mmr_score,
			missing_embedding: candidate.diversity_missing_embedding.unwrap_or(false),
		};
		let replace = match out.get(&candidate.note_id) {
			None => true,
			Some(existing) =>
				if decision.selected != existing.selected {
					decision.selected
				} else {
					let lhs = decision.selected_rank.unwrap_or(u32::MAX);
					let rhs = existing.selected_rank.unwrap_or(u32::MAX);

					lhs < rhs
				},
		};

		if replace {
			out.insert(candidate.note_id, decision);
		}
	}

	out
}

fn build_rerank_ranks(items: &[ChunkSnippet], scores: &[f32]) -> Vec<u32> {
	let n = items.len();

	if n == 0 {
		return Vec::new();
	}

	let mut idxs: Vec<usize> = (0..n).collect();

	idxs.sort_by(|&a, &b| {
		let score_a = scores.get(a).copied().unwrap_or(f32::NAN);
		let score_b = scores.get(b).copied().unwrap_or(f32::NAN);
		let ord = cmp_f32_desc(score_a, score_b);

		if ord != Ordering::Equal {
			return ord;
		}
		if items[a].note.note_id == items[b].note.note_id {
			let ord = items[a].chunk.chunk_index.cmp(&items[b].chunk.chunk_index);

			if ord != Ordering::Equal {
				return ord;
			}
		}

		let ord = items[a].retrieval_rank.cmp(&items[b].retrieval_rank);

		if ord != Ordering::Equal {
			return ord;
		}
		items[a].chunk.chunk_id.cmp(&items[b].chunk.chunk_id)
	});

	let mut ranks = vec![0_u32; n];

	for (pos, idx) in idxs.into_iter().enumerate() {
		ranks[idx] = pos as u32 + 1;
	}

	ranks
}

fn build_rerank_ranks_for_replay(candidates: &[TraceReplayCandidate]) -> Vec<u32> {
	let n = candidates.len();

	if n == 0 {
		return Vec::new();
	}

	let mut idxs: Vec<usize> = (0..n).collect();

	idxs.sort_by(|&a, &b| {
		let score_a = candidates.get(a).map(|candidate| candidate.rerank_score).unwrap_or(f32::NAN);
		let score_b = candidates.get(b).map(|candidate| candidate.rerank_score).unwrap_or(f32::NAN);
		let ord = cmp_f32_desc(score_a, score_b);

		if ord != Ordering::Equal {
			return ord;
		}

		let ra = candidates.get(a).map(|candidate| candidate.retrieval_rank).unwrap_or(0);
		let rb = candidates.get(b).map(|candidate| candidate.retrieval_rank).unwrap_or(0);
		let ord = ra.cmp(&rb);

		if ord != Ordering::Equal {
			return ord;
		}

		let na = candidates.get(a).map(|candidate| candidate.note_id).unwrap_or(Uuid::nil());
		let nb = candidates.get(b).map(|candidate| candidate.note_id).unwrap_or(Uuid::nil());
		let ord = na.cmp(&nb);

		if ord != Ordering::Equal {
			return ord;
		}

		let ca = candidates.get(a).map(|candidate| candidate.chunk_id).unwrap_or(Uuid::nil());
		let cb = candidates.get(b).map(|candidate| candidate.chunk_id).unwrap_or(Uuid::nil());

		ca.cmp(&cb)
	});

	let mut ranks = vec![0_u32; n];

	for (pos, idx) in idxs.into_iter().enumerate() {
		ranks[idx] = pos as u32 + 1;
	}

	ranks
}

fn cmp_f32_desc(a: f32, b: f32) -> Ordering {
	match (a.is_nan(), b.is_nan()) {
		(true, true) => Ordering::Equal,
		(true, false) => Ordering::Greater,
		(false, true) => Ordering::Less,
		(false, false) => b.partial_cmp(&a).unwrap_or(Ordering::Equal),
	}
}

fn resolve_scopes(cfg: &Config, profile: &str) -> Result<Vec<String>> {
	match profile {
		"private_only" => Ok(cfg.scopes.read_profiles.private_only.clone()),
		"private_plus_project" => Ok(cfg.scopes.read_profiles.private_plus_project.clone()),
		"all_scopes" => Ok(cfg.scopes.read_profiles.all_scopes.clone()),
		_ => Err(Error::InvalidRequest { message: "Unknown read_profile.".to_string() }),
	}
}

fn point_id_to_uuid(point_id: &qdrant_client::qdrant::PointId) -> Option<Uuid> {
	match &point_id.point_id_options {
		Some(PointIdOptions::Uuid(id)) => Uuid::parse_str(id).ok(),
		_ => None,
	}
}

fn payload_uuid(payload: &HashMap<String, Value>, key: &str) -> Option<Uuid> {
	let value = payload.get(key)?;

	match &value.kind {
		Some(Kind::StringValue(text)) => Uuid::parse_str(text).ok(),
		_ => None,
	}
}

fn payload_string(payload: &HashMap<String, Value>, key: &str) -> Option<String> {
	let value = payload.get(key)?;

	match &value.kind {
		Some(Kind::StringValue(text)) => Some(text.to_string()),
		_ => None,
	}
}

fn payload_rfc3339(payload: &HashMap<String, Value>, key: &str) -> Option<OffsetDateTime> {
	let text = payload_string(payload, key)?;

	OffsetDateTime::parse(text.as_str(), &Rfc3339).ok()
}

fn payload_i32(payload: &HashMap<String, Value>, key: &str) -> Option<i32> {
	let value = payload.get(key)?;

	match &value.kind {
		Some(Kind::IntegerValue(value)) => i32::try_from(*value).ok(),
		Some(Kind::DoubleValue(value)) =>
			if value.fract() == 0.0 {
				i32::try_from(*value as i64).ok()
			} else {
				None
			},
		_ => None,
	}
}

fn hash_query(query: &str) -> String {
	let mut hasher = DefaultHasher::new();

	Hash::hash(query, &mut hasher);

	format!("{:x}", hasher.finish())
}

fn hash_cache_key(payload: &serde_json::Value) -> Result<String> {
	let raw = serde_json::to_vec(payload).map_err(|err| Error::Storage {
		message: format!("Failed to encode cache key payload: {err}"),
	})?;

	Ok(blake3::hash(&raw).to_hex().to_string())
}

fn cache_key_prefix(key: &str) -> &str {
	let len = key.len().min(12);

	&key[..len]
}

fn build_expansion_cache_key(
	query: &str,
	max_queries: u32,
	include_original: bool,
	provider_id: &str,
	model: &str,
	temperature: f32,
) -> Result<String> {
	let payload = serde_json::json!({
		"kind": "expansion",
		"schema_version": EXPANSION_CACHE_SCHEMA_VERSION,
		"query": query.trim(),
		"provider_id": provider_id,
		"model": model,
		"temperature": temperature,
		"max_queries": max_queries,
		"include_original": include_original,
	});

	hash_cache_key(&payload)
}

fn build_rerank_cache_key(
	query: &str,
	provider_id: &str,
	model: &str,
	candidates: &[(Uuid, OffsetDateTime)],
) -> Result<String> {
	let signature: Vec<serde_json::Value> = candidates
		.iter()
		.map(|(chunk_id, updated_at)| {
			serde_json::json!({
				"chunk_id": chunk_id,
				"updated_at": updated_at,
			})
		})
		.collect();
	let payload = serde_json::json!({
		"kind": "rerank",
		"schema_version": RERANK_CACHE_SCHEMA_VERSION,
		"query": query.trim(),
		"provider_id": provider_id,
		"model": model,
		"candidates": signature,
	});

	hash_cache_key(&payload)
}

fn build_cached_scores(
	payload: &RerankCachePayload,
	candidates: &[RerankCacheCandidate],
) -> Option<Vec<f32>> {
	if payload.items.len() != candidates.len() {
		return None;
	}

	let mut map = HashMap::new();

	for item in &payload.items {
		let key = (item.chunk_id, item.updated_at.unix_timestamp(), item.updated_at.nanosecond());

		map.insert(key, item.score);
	}

	let mut out = Vec::with_capacity(candidates.len());

	for candidate in candidates {
		let key = (
			candidate.chunk_id,
			candidate.updated_at.unix_timestamp(),
			candidate.updated_at.nanosecond(),
		);
		let score = map.get(&key)?;

		out.push(*score);
	}

	Some(out)
}

async fn fetch_chunks_by_pair<'e, E>(executor: E, pairs: &[(Uuid, i32)]) -> Result<Vec<ChunkRow>>
where
	E: PgExecutor<'e>,
{
	if pairs.is_empty() {
		return Ok(Vec::new());
	}

	let mut builder = QueryBuilder::new(
		"SELECT chunk_id, note_id, chunk_index, start_offset, end_offset, text \
				FROM memory_note_chunks WHERE ",
	);
	let mut separated = builder.separated(" OR ");

	for (note_id, chunk_index) in pairs {
		separated.push("(");
		separated
			.push_unseparated("note_id = ")
			.push_bind_unseparated(note_id)
			.push_unseparated(" AND chunk_index = ")
			.push_bind_unseparated(chunk_index)
			.push_unseparated(")");
	}

	let query = builder.build_query_as();
	let rows = query.fetch_all(executor).await?;

	Ok(rows)
}

async fn fetch_note_vectors_for_diversity<'e, E>(
	executor: E,
	scored: &[ScoredChunk],
) -> Result<HashMap<Uuid, Vec<f32>>>
where
	E: PgExecutor<'e>,
{
	if scored.is_empty() {
		return Ok(HashMap::new());
	}

	let mut note_ids = Vec::new();
	let mut embedding_versions = Vec::new();
	let mut seen = HashSet::new();

	for scored_chunk in scored {
		let note_id = scored_chunk.item.note.note_id;

		if seen.insert(note_id) {
			note_ids.push(note_id);
			embedding_versions.push(scored_chunk.item.note.embedding_version.clone());
		}
	}

	let rows = sqlx::query_as::<_, NoteVectorRow>(
		"\
WITH expected AS (
	SELECT *
	FROM unnest($1::uuid[], $2::text[]) AS t(note_id, embedding_version)
)
SELECT
	e.note_id AS note_id,
	n.vec::text AS vec_text
FROM expected e
JOIN note_embeddings n
	ON n.note_id = e.note_id
	AND n.embedding_version = e.embedding_version",
	)
	.bind(note_ids.as_slice())
	.bind(embedding_versions.as_slice())
	.fetch_all(executor)
	.await?;

	let mut out = HashMap::new();

	for row in rows {
		let vec = crate::parse_pg_vector(row.vec_text.as_str())?;
		out.insert(row.note_id, vec);
	}

	Ok(out)
}

async fn enqueue_trace<'e, E>(executor: E, payload: TracePayload) -> Result<()>
where
	E: PgExecutor<'e>,
{
	let now = OffsetDateTime::now_utc();
	let payload_json = serde_json::to_value(&payload).map_err(|err| Error::Storage {
		message: format!("Failed to encode search trace payload: {err}"),
	})?;

	sqlx::query!(
		"\
INSERT INTO search_trace_outbox (
	outbox_id,
	trace_id,
	status,
	attempts,
	last_error,
	available_at,
	payload,
	created_at,
	updated_at
)
VALUES ($1, $2, 'PENDING', 0, NULL, $3, $4, $3, $3)",
		Uuid::new_v4(),
		payload.trace.trace_id,
		now,
		payload_json,
	)
	.execute(executor)
	.await?;

	Ok(())
}

async fn persist_trace_inline(
	executor: &mut sqlx::PgConnection,
	payload: TracePayload,
) -> Result<()> {
	let trace = payload.trace;
	let items = payload.items;
	let candidates = payload.candidates;
	let trace_id = trace.trace_id;
	let expanded_queries_json = serde_json::to_value(&trace.expanded_queries).map_err(|err| {
		Error::Storage { message: format!("Failed to encode expanded_queries: {err}") }
	})?;
	let allowed_scopes_json = serde_json::to_value(&trace.allowed_scopes).map_err(|err| {
		Error::Storage { message: format!("Failed to encode allowed_scopes: {err}") }
	})?;

	sqlx::query!(
		"\
INSERT INTO search_traces (
	trace_id,
	tenant_id,
	project_id,
	agent_id,
	read_profile,
	query,
	expansion_mode,
	expanded_queries,
	allowed_scopes,
	candidate_count,
	top_k,
	config_snapshot,
	trace_version,
	created_at,
	expires_at
)
VALUES (
	$1,
	$2,
	$3,
	$4,
	$5,
	$6,
	$7,
	$8,
	$9,
	$10,
	$11,
	$12,
	$13,
	$14,
	$15
)
ON CONFLICT (trace_id) DO NOTHING",
		trace_id,
		trace.tenant_id,
		trace.project_id,
		trace.agent_id,
		trace.read_profile,
		trace.query,
		trace.expansion_mode,
		expanded_queries_json,
		allowed_scopes_json,
		trace.candidate_count as i32,
		trace.top_k as i32,
		trace.config_snapshot,
		trace.trace_version,
		trace.created_at,
		trace.expires_at,
	)
	.execute(&mut *executor)
	.await?;

	if !items.is_empty() {
		let mut builder = QueryBuilder::new(
			"\
INSERT INTO search_trace_items (
	item_id,
	trace_id,
	note_id,
	chunk_id,
	rank,
	final_score,
	explain
) ",
		);
		builder.push_values(items, |mut b, item| {
			let explain_json = serde_json::to_value(item.explain)
				.expect("SearchExplain must be JSON-serializable.");

			b.push_bind(item.item_id)
				.push_bind(trace_id)
				.push_bind(item.note_id)
				.push_bind(item.chunk_id)
				.push_bind(item.rank as i32)
				.push_bind(item.final_score)
				.push_bind(explain_json);
		});
		builder.push(" ON CONFLICT (item_id) DO NOTHING");
		builder.build().execute(&mut *executor).await?;
	}

	if !candidates.is_empty() {
		let mut builder = QueryBuilder::new(
			"\
INSERT INTO search_trace_candidates (
	candidate_id,
	trace_id,
	note_id,
	chunk_id,
	chunk_index,
	snippet,
	candidate_snapshot,
	retrieval_rank,
	rerank_score,
	note_scope,
	note_importance,
	note_updated_at,
	note_hit_count,
	note_last_hit_at,
	created_at,
	expires_at
) ",
		);
		builder.push_values(candidates, |mut b, candidate| {
			b.push_bind(candidate.candidate_id)
				.push_bind(trace_id)
				.push_bind(candidate.note_id)
				.push_bind(candidate.chunk_id)
				.push_bind(candidate.chunk_index)
				.push_bind(candidate.snippet)
				.push_bind(candidate.candidate_snapshot)
				.push_bind(candidate.retrieval_rank as i32)
				.push_bind(candidate.rerank_score)
				.push_bind(candidate.note_scope)
				.push_bind(candidate.note_importance)
				.push_bind(candidate.note_updated_at)
				.push_bind(candidate.note_hit_count)
				.push_bind(candidate.note_last_hit_at)
				.push_bind(candidate.created_at)
				.push_bind(candidate.expires_at);
		});
		builder.push(" ON CONFLICT (candidate_id) DO NOTHING");
		builder.build().execute(&mut *executor).await?;
	}

	Ok(())
}

async fn record_hits<'e, E>(
	executor: E,
	query: &str,
	scored: &[ScoredChunk],
	now: OffsetDateTime,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	if scored.is_empty() {
		return Ok(());
	}

	let query_hash = hash_query(query);
	let mut hit_ids = Vec::with_capacity(scored.len());
	let mut note_ids = Vec::with_capacity(scored.len());
	let mut chunk_ids = Vec::with_capacity(scored.len());
	let mut ranks = Vec::with_capacity(scored.len());
	let mut final_scores = Vec::with_capacity(scored.len());

	for (rank, scored_chunk) in scored.iter().enumerate() {
		hit_ids.push(Uuid::new_v4());
		note_ids.push(scored_chunk.item.note.note_id);
		chunk_ids.push(scored_chunk.item.chunk.chunk_id);
		ranks.push(rank as i32);
		final_scores.push(scored_chunk.final_score);
	}

	sqlx::query!(
		"\
WITH hits AS (
	SELECT *
	FROM unnest(
		$1::uuid[],
		$2::uuid[],
		$3::uuid[],
		$4::int4[],
		$5::real[]
	) AS t(hit_id, note_id, chunk_id, rank, final_score)
),
updated AS (
	UPDATE memory_notes
	SET
		hit_count = hit_count + 1,
		last_hit_at = $6
	WHERE note_id = ANY($2)
)
INSERT INTO memory_hits (
	hit_id,
	note_id,
	chunk_id,
	query_hash,
	rank,
	final_score,
	ts
)
SELECT
	hit_id,
	note_id,
	chunk_id,
	$7,
	rank,
	final_score,
	$6
FROM hits",
		&hit_ids,
		&note_ids,
		&chunk_ids,
		&ranks,
		&final_scores,
		now,
		query_hash.as_str(),
	)
	.execute(executor)
	.await?;

	Ok(())
}

async fn fetch_cache_payload<'e, E>(
	executor: E,
	kind: CacheKind,
	key: &str,
	now: OffsetDateTime,
) -> Result<Option<CachePayload>>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query!(
		"\
WITH updated AS (
	UPDATE llm_cache
	SET
		last_accessed_at = $3,
		hit_count = hit_count + 1
	WHERE
		cache_kind = $1
		AND cache_key = $2
		AND expires_at > $3
	RETURNING payload
)
SELECT payload
FROM updated",
		kind.as_str(),
		key,
		now,
	)
	.fetch_optional(executor)
	.await?;
	let Some(row) = row else {
		return Ok(None);
	};
	let payload = row.payload;

	let size_bytes = serde_json::to_vec(&payload)
		.map_err(|err| Error::Storage {
			message: format!("Failed to encode cache payload: {err}"),
		})?
		.len();

	Ok(Some(CachePayload { value: payload, size_bytes }))
}

async fn store_cache_payload<'e, E>(
	executor: E,
	kind: CacheKind,
	key: &str,
	payload: serde_json::Value,
	now: OffsetDateTime,
	expires_at: OffsetDateTime,
	max_payload_bytes: Option<u64>,
) -> Result<Option<usize>>
where
	E: PgExecutor<'e>,
{
	let payload_bytes = serde_json::to_vec(&payload).map_err(|err| Error::Storage {
		message: format!("Failed to encode cache payload: {err}"),
	})?;
	let payload_size = payload_bytes.len();

	if let Some(max) = max_payload_bytes
		&& payload_size as u64 > max
	{
		return Ok(None);
	}

	sqlx::query!(
		"\
INSERT INTO llm_cache (
	cache_id,
	cache_kind,
	cache_key,
	payload,
	created_at,
	last_accessed_at,
	expires_at,
	hit_count
)
VALUES ($1, $2, $3, $4, $5, $5, $6, 0)
ON CONFLICT (cache_kind, cache_key) DO UPDATE SET
payload = EXCLUDED.payload,
	last_accessed_at = EXCLUDED.last_accessed_at,
	expires_at = EXCLUDED.expires_at,
	hit_count = 0",
		Uuid::new_v4(),
		kind.as_str(),
		key,
		payload,
		now,
		expires_at,
	)
	.execute(executor)
	.await?;

	Ok(Some(payload_size))
}

#[cfg(test)]
mod tests {
	use super::*;
	use elf_config::{Config, SearchDynamic};

	#[test]
	fn dense_embedding_input_includes_project_context_suffix() {
		let input =
			build_dense_embedding_input("Find payments code.", Some("This is a billing API."));

		assert!(input.starts_with("Find payments code.\n\nProject context:\n"));
		assert!(input.contains("This is a billing API."));
	}

	#[test]
	fn dense_embedding_input_skips_empty_project_context() {
		let input = build_dense_embedding_input("Find payments code.", Some("   "));

		assert_eq!(input, "Find payments code.");
	}

	#[test]
	fn scope_description_boost_matches_whole_tokens_only() {
		let tokens = vec!["go".to_string()];
		let boost = scope_description_boost(&tokens, "MongoDB operational notes.", 0.1);

		assert_eq!(boost, 0.0);
	}

	#[test]
	fn scope_description_boost_scales_by_fraction_of_matched_tokens() {
		let tokens = vec!["security".to_string(), "policy".to_string(), "deployment".to_string()];
		let boost = scope_description_boost(&tokens, "Security policy notes.", 0.12);

		assert!((boost - 0.08).abs() < 1e-4, "Unexpected boost: {boost}");
	}

	#[test]
	fn normalize_queries_includes_original_and_dedupes() {
		let queries = vec!["alpha".to_string(), "beta".to_string(), "alpha".to_string()];
		let normalized = normalize_queries(queries, "alpha", true, 4);

		assert_eq!(normalized, vec!["alpha".to_string(), "beta".to_string()]);
	}

	#[test]
	fn normalize_queries_respects_max_queries() {
		let queries =
			vec!["one".to_string(), "two".to_string(), "three".to_string(), "four".to_string()];
		let normalized = normalize_queries(queries, "zero", true, 3);

		assert_eq!(normalized.len(), 3);
	}

	#[test]
	fn dynamic_trigger_checks_candidates_and_score() {
		let cfg = SearchDynamic { min_candidates: 10, min_top_score: 0.2 };

		assert!(should_expand_dynamic(5, 0.9, &cfg));
		assert!(should_expand_dynamic(20, 0.1, &cfg));
		assert!(!should_expand_dynamic(20, 0.9, &cfg));
	}

	#[test]
	fn rank_normalize_maps_rank_to_unit_interval() {
		assert!((rank_normalize(1, 1) - 1.0).abs() < 1e-6);
		assert!((rank_normalize(1, 5) - 1.0).abs() < 1e-6);
		assert!((rank_normalize(3, 5) - 0.5).abs() < 1e-6);
		assert!((rank_normalize(5, 5) - 0.0).abs() < 1e-6);
		assert!((rank_normalize(0, 5) - 0.0).abs() < 1e-6);
	}

	fn test_chunk_candidate(note_id: Uuid, retrieval_rank: u32) -> ChunkCandidate {
		ChunkCandidate {
			chunk_id: Uuid::new_v4(),
			note_id,
			chunk_index: 0,
			retrieval_rank,
			updated_at: None,
			embedding_version: Some("v1".to_string()),
		}
	}

	fn default_retrieval_sources_policy() -> ResolvedRetrievalSourcesPolicy {
		ResolvedRetrievalSourcesPolicy {
			fusion_weight: 1.0,
			structured_field_weight: 1.0,
			fusion_priority: 1,
			structured_field_priority: 0,
		}
	}

	#[test]
	fn merge_retrieval_candidates_keeps_structured_hits_under_full_fusion_capacity() {
		let mut fusion = Vec::new();

		for rank in 1..=10 {
			fusion.push(test_chunk_candidate(Uuid::new_v4(), rank));
		}

		let structured = vec![test_chunk_candidate(Uuid::new_v4(), 1)];
		let structured_chunk_id = structured[0].chunk_id;
		let merged = merge_retrieval_candidates(
			vec![
				RetrievalSourceCandidates {
					source: RetrievalSourceKind::Fusion,
					candidates: fusion,
				},
				RetrievalSourceCandidates {
					source: RetrievalSourceKind::StructuredField,
					candidates: structured,
				},
			],
			&default_retrieval_sources_policy(),
			10,
		);
		let merged_chunk_ids: Vec<Uuid> =
			merged.iter().map(|candidate| candidate.chunk_id).collect();

		assert!(
			merged_chunk_ids.contains(&structured_chunk_id),
			"Structured candidate was dropped by retrieval fusion."
		);
	}

	#[test]
	fn merge_retrieval_candidates_prefers_dual_source_signal_on_tie() {
		let shared_note_id = Uuid::new_v4();
		let shared_chunk_id = Uuid::new_v4();
		let fusion_only_note_id = Uuid::new_v4();
		let fusion_only_chunk_id = Uuid::new_v4();
		let fusion = vec![
			ChunkCandidate {
				chunk_id: shared_chunk_id,
				note_id: shared_note_id,
				chunk_index: 0,
				retrieval_rank: 9,
				updated_at: None,
				embedding_version: Some("v1".to_string()),
			},
			ChunkCandidate {
				chunk_id: fusion_only_chunk_id,
				note_id: fusion_only_note_id,
				chunk_index: 0,
				retrieval_rank: 1,
				updated_at: None,
				embedding_version: Some("v1".to_string()),
			},
		];
		let structured = vec![ChunkCandidate {
			chunk_id: shared_chunk_id,
			note_id: shared_note_id,
			chunk_index: 0,
			retrieval_rank: 1,
			updated_at: None,
			embedding_version: Some("v1".to_string()),
		}];
		let merged = merge_retrieval_candidates(
			vec![
				RetrievalSourceCandidates {
					source: RetrievalSourceKind::Fusion,
					candidates: fusion,
				},
				RetrievalSourceCandidates {
					source: RetrievalSourceKind::StructuredField,
					candidates: structured,
				},
			],
			&default_retrieval_sources_policy(),
			1,
		);
		let first = merged.first().expect("Expected merged candidate.");

		assert_eq!(first.chunk_id, shared_chunk_id);
	}

	#[test]
	fn retrieval_weight_for_rank_uses_first_matching_segment_or_last() {
		let segments = vec![
			BlendSegment { max_retrieval_rank: 3, retrieval_weight: 0.7 },
			BlendSegment { max_retrieval_rank: 10, retrieval_weight: 0.2 },
		];

		assert!((retrieval_weight_for_rank(1, &segments) - 0.7).abs() < 1e-6);
		assert!((retrieval_weight_for_rank(3, &segments) - 0.7).abs() < 1e-6);
		assert!((retrieval_weight_for_rank(4, &segments) - 0.2).abs() < 1e-6);
		assert!((retrieval_weight_for_rank(999, &segments) - 0.2).abs() < 1e-6);
	}

	#[test]
	fn blend_math_is_linear_and_additive() {
		let segments = vec![
			BlendSegment { max_retrieval_rank: 2, retrieval_weight: 0.7 },
			BlendSegment { max_retrieval_rank: 10, retrieval_weight: 0.2 },
		];
		let retrieval_rank = 3;
		let rerank_rank = 2;
		let retrieval_norm = rank_normalize(retrieval_rank, 10);
		let rerank_norm = rank_normalize(rerank_rank, 4);
		let blend_retrieval_weight = retrieval_weight_for_rank(retrieval_rank, &segments);

		assert!((blend_retrieval_weight - 0.2).abs() < 1e-6);
		assert!((retrieval_norm - (7.0 / 9.0)).abs() < 1e-6);
		assert!((rerank_norm - (2.0 / 3.0)).abs() < 1e-6);

		let retrieval_term = blend_retrieval_weight * retrieval_norm;
		let rerank_term = (1.0 - blend_retrieval_weight) * rerank_norm;
		let tie_breaker_score = 0.1;
		let scope_context_boost = 0.0;
		let final_score = retrieval_term + rerank_term + tie_breaker_score + scope_context_boost;
		let expected = (0.2 * (7.0 / 9.0)) + (0.8 * (2.0 / 3.0)) + 0.1;

		assert!((final_score - expected).abs() < 1e-6, "Unexpected final_score: {final_score}");
	}

	#[test]
	fn expansion_cache_key_changes_with_max_queries() {
		let key_a = build_expansion_cache_key("alpha", 4, true, "llm", "model", 0.1_f32)
			.expect("Expected cache key.");
		let key_b = build_expansion_cache_key("alpha", 5, true, "llm", "model", 0.1_f32)
			.expect("Expected cache key.");

		assert_ne!(key_a, key_b);
	}

	#[test]
	fn rerank_cache_key_changes_with_updated_at() {
		let ts_a = OffsetDateTime::from_unix_timestamp(1).expect("Valid timestamp.");
		let ts_b = OffsetDateTime::from_unix_timestamp(2).expect("Valid timestamp.");
		let chunk_id = Uuid::new_v4();
		let key_a = build_rerank_cache_key("q", "rerank", "model", &[(chunk_id, ts_a)])
			.expect("Expected cache key.");
		let key_b = build_rerank_cache_key("q", "rerank", "model", &[(chunk_id, ts_b)])
			.expect("Expected cache key.");

		assert_ne!(key_a, key_b);
	}

	#[test]
	fn rerank_cache_payload_rejects_mismatched_counts() {
		let payload = RerankCachePayload {
			items: vec![RerankCacheItem {
				chunk_id: Uuid::new_v4(),
				updated_at: OffsetDateTime::from_unix_timestamp(1).expect("Valid timestamp."),
				score: 0.5,
			}],
		};
		let candidates = vec![RerankCacheCandidate {
			chunk_id: Uuid::new_v4(),
			updated_at: OffsetDateTime::from_unix_timestamp(1).expect("Valid timestamp."),
		}];

		assert!(build_cached_scores(&payload, &candidates).is_none());
	}

	#[test]
	fn cache_key_prefix_is_stable() {
		let prefix = cache_key_prefix("abcd1234efgh5678");

		assert_eq!(prefix, "abcd1234efgh");
	}

	#[test]
	fn lexical_overlap_ratio_is_deterministic_and_bounded() {
		let query_tokens = vec!["deploy".to_string(), "steps".to_string()];
		let ratio = lexical_overlap_ratio(&query_tokens, "Deploy steps for staging.", 128);

		assert!((ratio - 1.0).abs() < 1e-6, "Unexpected ratio: {ratio}");

		let ratio = lexical_overlap_ratio(&query_tokens, "Deploy only.", 128);

		assert!((ratio - 0.5).abs() < 1e-6, "Unexpected ratio: {ratio}");
		assert!((0.0..=1.0).contains(&ratio), "Ratio must be in [0, 1].");
	}

	#[test]
	fn deterministic_ranking_terms_do_not_apply_when_disabled() {
		let mut cfg = parse_example_config();
		cfg.ranking.deterministic.enabled = false;
		cfg.ranking.deterministic.lexical.enabled = true;
		cfg.ranking.deterministic.hits.enabled = true;
		cfg.ranking.deterministic.decay.enabled = true;

		let now = OffsetDateTime::from_unix_timestamp(1_000_000).expect("Valid timestamp.");
		let note = NoteMeta {
			note_id: Uuid::new_v4(),
			note_type: "fact".to_string(),
			key: None,
			scope: "project_shared".to_string(),
			importance: 0.1,
			confidence: 0.9,
			updated_at: now,
			expires_at: None,
			source_ref: serde_json::json!({}),
			embedding_version: "v1".to_string(),
			hit_count: 8,
			last_hit_at: Some(now),
		};
		let chunk =
			ChunkMeta { chunk_id: Uuid::new_v4(), chunk_index: 0, start_offset: 0, end_offset: 10 };
		let item =
			ChunkSnippet { note, chunk, snippet: "deploy steps".to_string(), retrieval_rank: 1 };
		let mut scored = ScoredChunk {
			item,
			final_score: 1.0,
			rerank_score: 0.5,
			rerank_rank: 1,
			rerank_norm: 1.0,
			retrieval_norm: 1.0,
			blend_retrieval_weight: 0.5,
			retrieval_term: 0.5,
			rerank_term: 0.5,
			tie_breaker_score: 0.0,
			scope_context_boost: 0.0,
			age_days: 30.0,
			importance: 0.1,
			deterministic_lexical_overlap_ratio: 0.0,
			deterministic_lexical_bonus: 0.0,
			deterministic_hit_count: 0,
			deterministic_last_hit_age_days: None,
			deterministic_hit_boost: 0.0,
			deterministic_decay_penalty: 0.0,
		};
		let terms = compute_deterministic_ranking_terms(
			&cfg,
			&tokenize_query(
				"deploy steps",
				cfg.ranking.deterministic.lexical.max_query_terms as usize,
			),
			scored.item.snippet.as_str(),
			scored.item.note.hit_count,
			scored.item.note.last_hit_at,
			scored.age_days,
			now,
		);

		scored.final_score += terms.lexical_bonus + terms.hit_boost + terms.decay_penalty;
		scored.deterministic_lexical_overlap_ratio = terms.lexical_overlap_ratio;
		scored.deterministic_lexical_bonus = terms.lexical_bonus;
		scored.deterministic_hit_count = terms.hit_count;
		scored.deterministic_last_hit_age_days = terms.last_hit_age_days;
		scored.deterministic_hit_boost = terms.hit_boost;
		scored.deterministic_decay_penalty = terms.decay_penalty;

		assert!((scored.final_score - 1.0).abs() < 1e-6, "Score must not change.");
		assert!((scored.deterministic_lexical_bonus - 0.0).abs() < 1e-6);
		assert!((scored.deterministic_hit_boost - 0.0).abs() < 1e-6);
		assert!((scored.deterministic_decay_penalty - 0.0).abs() < 1e-6);
	}

	#[test]
	fn deterministic_ranking_terms_apply_and_are_bounded() {
		let mut cfg = parse_example_config();

		cfg.ranking.deterministic.enabled = true;
		cfg.ranking.deterministic.lexical.enabled = true;
		cfg.ranking.deterministic.hits.enabled = true;
		cfg.ranking.deterministic.decay.enabled = true;

		let now = OffsetDateTime::from_unix_timestamp(1_000_000).expect("Valid timestamp.");
		let note = NoteMeta {
			note_id: Uuid::new_v4(),
			note_type: "fact".to_string(),
			key: None,
			scope: "project_shared".to_string(),
			importance: 0.1,
			confidence: 0.9,
			updated_at: now,
			expires_at: None,
			source_ref: serde_json::json!({}),
			embedding_version: "v1".to_string(),
			hit_count: 8,
			last_hit_at: Some(now),
		};
		let chunk =
			ChunkMeta { chunk_id: Uuid::new_v4(), chunk_index: 0, start_offset: 0, end_offset: 10 };
		let item =
			ChunkSnippet { note, chunk, snippet: "deploy steps".to_string(), retrieval_rank: 1 };
		let mut scored = ScoredChunk {
			item,
			final_score: 1.0,
			rerank_score: 0.5,
			rerank_rank: 1,
			rerank_norm: 1.0,
			retrieval_norm: 1.0,
			blend_retrieval_weight: 0.5,
			retrieval_term: 0.5,
			rerank_term: 0.5,
			tie_breaker_score: 0.0,
			scope_context_boost: 0.0,
			age_days: 30.0,
			importance: 0.1,
			deterministic_lexical_overlap_ratio: 0.0,
			deterministic_lexical_bonus: 0.0,
			deterministic_hit_count: 0,
			deterministic_last_hit_age_days: None,
			deterministic_hit_boost: 0.0,
			deterministic_decay_penalty: 0.0,
		};
		let terms = compute_deterministic_ranking_terms(
			&cfg,
			&tokenize_query(
				"deploy steps",
				cfg.ranking.deterministic.lexical.max_query_terms as usize,
			),
			scored.item.snippet.as_str(),
			scored.item.note.hit_count,
			scored.item.note.last_hit_at,
			scored.age_days,
			now,
		);

		scored.final_score += terms.lexical_bonus + terms.hit_boost + terms.decay_penalty;
		scored.deterministic_lexical_overlap_ratio = terms.lexical_overlap_ratio;
		scored.deterministic_lexical_bonus = terms.lexical_bonus;
		scored.deterministic_hit_count = terms.hit_count;
		scored.deterministic_last_hit_age_days = terms.last_hit_age_days;
		scored.deterministic_hit_boost = terms.hit_boost;
		scored.deterministic_decay_penalty = terms.decay_penalty;

		assert!(scored.final_score.is_finite(), "Score must be finite.");
		assert!((0.0..=1.0).contains(&scored.deterministic_lexical_overlap_ratio));
		assert!(scored.deterministic_lexical_bonus >= 0.0);
		assert!(scored.deterministic_hit_boost >= 0.0);
		assert!(scored.deterministic_decay_penalty <= 0.0);

		let expected_lex = cfg.ranking.deterministic.lexical.weight;

		assert!((scored.deterministic_lexical_bonus - expected_lex).abs() < 1e-6);

		let expected_hit = cfg.ranking.deterministic.hits.weight * 0.5;

		assert!((scored.deterministic_hit_boost - expected_hit).abs() < 1e-6);
	}

	fn test_scored_chunk(note_id: Uuid, retrieval_rank: u32, now: OffsetDateTime) -> ScoredChunk {
		let note = NoteMeta {
			note_id,
			note_type: "fact".to_string(),
			key: None,
			scope: "project_shared".to_string(),
			importance: 0.1,
			confidence: 0.9,
			updated_at: now,
			expires_at: None,
			source_ref: serde_json::json!({}),
			embedding_version: "v1".to_string(),
			hit_count: 0,
			last_hit_at: None,
		};
		let chunk = ChunkMeta {
			chunk_id: Uuid::new_v4(),
			chunk_index: i32::try_from(retrieval_rank.saturating_sub(1)).unwrap_or(0),
			start_offset: 0,
			end_offset: 16,
		};
		let item = ChunkSnippet {
			note,
			chunk,
			snippet: format!("snippet-{retrieval_rank}"),
			retrieval_rank,
		};

		ScoredChunk {
			item,
			final_score: 0.0,
			rerank_score: 0.0,
			rerank_rank: retrieval_rank,
			rerank_norm: 0.0,
			retrieval_norm: 0.0,
			blend_retrieval_weight: 0.5,
			retrieval_term: 0.0,
			rerank_term: 0.0,
			tie_breaker_score: 0.0,
			scope_context_boost: 0.0,
			age_days: 0.0,
			importance: 0.1,
			deterministic_lexical_overlap_ratio: 0.0,
			deterministic_lexical_bonus: 0.0,
			deterministic_hit_count: 0,
			deterministic_last_hit_age_days: None,
			deterministic_hit_boost: 0.0,
			deterministic_decay_penalty: 0.0,
		}
	}

	#[test]
	fn diversity_selection_skips_high_similarity_when_alternative_exists() {
		let now = OffsetDateTime::from_unix_timestamp(0).expect("Valid timestamp.");
		let note_a = Uuid::new_v4();
		let note_b = Uuid::new_v4();
		let note_c = Uuid::new_v4();
		let candidates = vec![
			test_scored_chunk(note_a, 1, now),
			test_scored_chunk(note_b, 2, now),
			test_scored_chunk(note_c, 3, now),
		];
		let mut vectors = HashMap::new();

		vectors.insert(note_a, vec![1.0, 0.0]);
		vectors.insert(note_b, vec![0.99, 0.01]);
		vectors.insert(note_c, vec![0.0, 1.0]);

		let policy = ResolvedDiversityPolicy {
			enabled: true,
			sim_threshold: 0.9,
			mmr_lambda: 0.7,
			max_skips: 64,
		};
		let (selected, decisions) = select_diverse_results(candidates, 2, &policy, &vectors);
		let selected_ids: Vec<Uuid> = selected.iter().map(|item| item.item.note.note_id).collect();

		assert_eq!(selected_ids, vec![note_a, note_c]);
		assert_eq!(
			decisions.get(&note_b).and_then(|decision| decision.skipped_reason.as_deref()),
			Some("similarity_threshold")
		);
	}

	#[test]
	fn diversity_selection_backfills_when_max_skips_is_reached() {
		let now = OffsetDateTime::from_unix_timestamp(0).expect("Valid timestamp.");
		let note_a = Uuid::new_v4();
		let note_b = Uuid::new_v4();
		let candidates = vec![test_scored_chunk(note_a, 1, now), test_scored_chunk(note_b, 2, now)];
		let mut vectors = HashMap::new();

		vectors.insert(note_a, vec![1.0, 0.0]);
		vectors.insert(note_b, vec![0.99, 0.01]);

		let policy = ResolvedDiversityPolicy {
			enabled: true,
			sim_threshold: 0.9,
			mmr_lambda: 0.7,
			max_skips: 0,
		};
		let (selected, decisions) = select_diverse_results(candidates, 2, &policy, &vectors);
		let selected_ids: Vec<Uuid> = selected.iter().map(|item| item.item.note.note_id).collect();
		let selected_reason =
			decisions.get(&note_b).map(|decision| decision.selected_reason.as_str());

		assert_eq!(selected_ids, vec![note_a, note_b]);
		assert_eq!(selected_reason, Some("max_skips_backfill"));
	}

	#[test]
	fn replay_diversity_decisions_prefer_selected_entry_for_same_note() {
		let now = OffsetDateTime::from_unix_timestamp(0).expect("Valid timestamp.");
		let note_id = Uuid::new_v4();
		let first = TraceReplayCandidate {
			note_id,
			chunk_id: Uuid::new_v4(),
			chunk_index: 0,
			snippet: "first".to_string(),
			retrieval_rank: 2,
			rerank_score: 0.2,
			note_scope: "project_shared".to_string(),
			note_importance: 0.1,
			note_updated_at: now,
			note_hit_count: 0,
			note_last_hit_at: None,
			diversity_selected: Some(false),
			diversity_selected_rank: None,
			diversity_selected_reason: Some("not_selected".to_string()),
			diversity_skipped_reason: Some("lower_mmr".to_string()),
			diversity_nearest_selected_note_id: None,
			diversity_similarity: Some(0.95),
			diversity_mmr_score: Some(0.12),
			diversity_missing_embedding: Some(false),
		};
		let second = TraceReplayCandidate {
			note_id,
			chunk_id: Uuid::new_v4(),
			chunk_index: 1,
			snippet: "second".to_string(),
			retrieval_rank: 1,
			rerank_score: 0.3,
			note_scope: "project_shared".to_string(),
			note_importance: 0.1,
			note_updated_at: now,
			note_hit_count: 0,
			note_last_hit_at: None,
			diversity_selected: Some(true),
			diversity_selected_rank: Some(2),
			diversity_selected_reason: Some("mmr".to_string()),
			diversity_skipped_reason: None,
			diversity_nearest_selected_note_id: None,
			diversity_similarity: Some(0.35),
			diversity_mmr_score: Some(0.44),
			diversity_missing_embedding: Some(false),
		};

		let decisions = extract_replay_diversity_decisions(&[first, second]);
		let decision = decisions.get(&note_id).expect("Expected merged decision.");

		assert!(decision.selected);
		assert_eq!(decision.selected_rank, Some(2));
		assert_eq!(decision.selected_reason, "mmr");
	}

	fn parse_example_config() -> Config {
		let root_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
		let path = root_dir.join("elf.example.toml");

		elf_config::load(&path).expect("elf.example.toml must remain parseable and valid.")
	}

	#[test]
	fn ranking_policy_id_is_stable_and_has_expected_format() {
		let cfg = parse_example_config();
		let id_a = ranking_policy_id(&cfg, None).expect("Expected policy id.");
		let id_b = ranking_policy_id(&cfg, None).expect("Expected policy id.");

		assert_eq!(id_a, id_b);
		assert!(id_a.starts_with("ranking_v2:"), "Unexpected policy id: {id_a}");
		assert_eq!(id_a.len(), "ranking_v2:".len() + 12, "Unexpected policy id: {id_a}");
	}

	#[test]
	fn ranking_policy_id_changes_with_override() {
		let cfg = parse_example_config();
		let base = ranking_policy_id(&cfg, None).expect("Expected base policy id.");
		let override_ = RankingRequestOverride {
			blend: Some(BlendRankingOverride {
				enabled: Some(false),
				rerank_normalization: None,
				retrieval_normalization: None,
				segments: None,
			}),
			diversity: None,
			retrieval_sources: None,
		};
		let overridden =
			ranking_policy_id(&cfg, Some(&override_)).expect("Expected overridden policy id.");

		assert_ne!(base, overridden);
	}

	#[test]
	fn ranking_policy_id_changes_with_retrieval_source_override() {
		let cfg = parse_example_config();
		let base = ranking_policy_id(&cfg, None).expect("Expected base policy id.");
		let override_ = RankingRequestOverride {
			blend: None,
			diversity: None,
			retrieval_sources: Some(RetrievalSourcesRankingOverride {
				fusion_weight: Some(0.75),
				structured_field_weight: Some(1.25),
				fusion_priority: Some(2),
				structured_field_priority: Some(1),
			}),
		};
		let overridden =
			ranking_policy_id(&cfg, Some(&override_)).expect("Expected overridden policy id.");

		assert_ne!(base, overridden);
	}

	#[test]
	fn replay_ranking_policy_id_matches_ranking_policy_id() {
		let cfg = parse_example_config();
		let expected = ranking_policy_id(&cfg, None).expect("Expected policy id.");
		let now = OffsetDateTime::from_unix_timestamp(0).expect("Valid timestamp.");
		let trace = TraceReplayContext {
			trace_id: Uuid::new_v4(),
			query: "deployment steps".to_string(),
			candidate_count: 3,
			top_k: 2,
			created_at: now,
		};
		let candidates = vec![
			TraceReplayCandidate {
				note_id: Uuid::new_v4(),
				chunk_id: Uuid::new_v4(),
				chunk_index: 0,
				snippet: "deployment steps".to_string(),
				retrieval_rank: 1,
				rerank_score: 0.1,
				note_scope: "project_shared".to_string(),
				note_importance: 0.1,
				note_updated_at: now,
				note_hit_count: 0,
				note_last_hit_at: None,
				diversity_selected: None,
				diversity_selected_rank: None,
				diversity_selected_reason: None,
				diversity_skipped_reason: None,
				diversity_nearest_selected_note_id: None,
				diversity_similarity: None,
				diversity_mmr_score: None,
				diversity_missing_embedding: None,
			},
			TraceReplayCandidate {
				note_id: Uuid::new_v4(),
				chunk_id: Uuid::new_v4(),
				chunk_index: 0,
				snippet: "deployment steps".to_string(),
				retrieval_rank: 2,
				rerank_score: 0.9,
				note_scope: "project_shared".to_string(),
				note_importance: 0.1,
				note_updated_at: now,
				note_hit_count: 0,
				note_last_hit_at: None,
				diversity_selected: None,
				diversity_selected_rank: None,
				diversity_selected_reason: None,
				diversity_skipped_reason: None,
				diversity_nearest_selected_note_id: None,
				diversity_similarity: None,
				diversity_mmr_score: None,
				diversity_missing_embedding: None,
			},
			TraceReplayCandidate {
				note_id: Uuid::new_v4(),
				chunk_id: Uuid::new_v4(),
				chunk_index: 0,
				snippet: "deployment steps".to_string(),
				retrieval_rank: 3,
				rerank_score: 0.2,
				note_scope: "org_shared".to_string(),
				note_importance: 0.1,
				note_updated_at: now,
				note_hit_count: 0,
				note_last_hit_at: None,
				diversity_selected: None,
				diversity_selected_rank: None,
				diversity_selected_reason: None,
				diversity_skipped_reason: None,
				diversity_nearest_selected_note_id: None,
				diversity_similarity: None,
				diversity_mmr_score: None,
				diversity_missing_embedding: None,
			},
		];
		let out = replay_ranking_from_candidates(&cfg, &trace, None, &candidates, 2)
			.expect("Expected replay output.");

		for item in out {
			assert_eq!(item.explain.ranking.policy_id, expected);
		}
	}
}
