mod ranking;

use std::{
	cmp::Ordering,
	collections::{BTreeMap, HashMap, HashSet},
	slice,
};

use qdrant_client::qdrant::{
	Condition, Document, Filter, Fusion, MinShould, PrefetchQueryBuilder, Query,
	QueryPointsBuilder, ScoredPoint,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, QueryBuilder};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub use crate::ranking_explain_v2::{SearchRankingExplain, SearchRankingTerm};
use crate::{ElfService, Error, Result, ranking_explain_v2};
use elf_config::Config;
use elf_domain::cjk;
use elf_storage::{
	models::MemoryNote,
	qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME},
};

const TRACE_VERSION: i32 = 2;
const MAX_MATCHED_TERMS: usize = 8;

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
	pub ranking: Option<RankingRequestOverride>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankingRequestOverride {
	pub blend: Option<BlendRankingOverride>,
	pub diversity: Option<DiversityRankingOverride>,
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
	#[serde(skip_serializing_if = "Option::is_none")]
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
	#[serde(skip_serializing_if = "Option::is_none")]
	pub skipped_reason: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub nearest_selected_note_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub similarity: Option<f32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mmr_score: Option<f32>,
	#[serde(default)]
	pub missing_embedding: bool,
}

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
	pub diversity_selected: Option<bool>,
	pub diversity_selected_rank: Option<u32>,
	pub diversity_selected_reason: Option<String>,
	pub diversity_skipped_reason: Option<String>,
	pub diversity_nearest_selected_note_id: Option<Uuid>,
	pub diversity_similarity: Option<f32>,
	pub diversity_mmr_score: Option<f32>,
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
			expansion_mode: ranking::expansion_mode_label(context.expansion_mode).to_string(),
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
		let retrieval_sources_policy = ranking::resolve_retrieval_sources_policy(
			&self.cfg.ranking.retrieval_sources,
			ranking_override.as_ref().and_then(|override_| override_.retrieval_sources.as_ref()),
		)?;
		let expansion_mode = ranking::resolve_expansion_mode(&self.cfg);
		let trace_id = Uuid::new_v4();
		let project_context_description =
			self.resolve_project_context_description(tenant_id, project_id);
		let allowed_scopes = ranking::resolve_scopes(&self.cfg, &read_profile)?;

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
			let candidates = ranking::collect_chunk_candidates(
				&baseline_points,
				self.cfg.search.prefilter.max_candidates,
				candidate_k,
			);
			let should_expand = ranking::should_expand_dynamic(
				baseline_points.len(),
				top_score,
				&self.cfg.search.dynamic,
			);

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
				let merged_candidates = ranking::merge_retrieval_candidates(
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
		let candidates = ranking::collect_chunk_candidates(
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
		let merged_candidates = ranking::merge_retrieval_candidates(
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
		let expanded_queries: Vec<String> =
			ranking::decode_json(row.expanded_queries, "expanded_queries")?;
		let allowed_scopes: Vec<String> =
			ranking::decode_json(row.allowed_scopes, "allowed_scopes")?;
		let config_snapshot = row.config_snapshot;
		let explain: SearchExplain = ranking::decode_json(row.explain, "explain")?;
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
		let expanded_queries: Vec<String> =
			ranking::decode_json(row.expanded_queries, "expanded_queries")?;
		let allowed_scopes: Vec<String> =
			ranking::decode_json(row.allowed_scopes, "allowed_scopes")?;
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
			let explain: SearchExplain = ranking::decode_json(row.explain, "explain")?;

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
		let input = ranking::build_dense_embedding_input(query, project_context_description);
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
			extra_inputs
				.push(ranking::build_dense_embedding_input(query, project_context_description));
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
			match ranking::build_expansion_cache_key(
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
						cache_key_prefix = ranking::cache_key_prefix(key),
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
								cache_key_prefix = ranking::cache_key_prefix(key),
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
						cache_key_prefix = ranking::cache_key_prefix(key),
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
						cache_key_prefix = ranking::cache_key_prefix(key),
						"Cache read failed."
					);
				},
			}
		}

		let messages =
			ranking::build_expansion_messages(query, cfg.max_queries, cfg.include_original);
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
		let normalized = ranking::normalize_queries(
			parsed.queries,
			query,
			cfg.include_original,
			cfg.max_queries,
		);
		let result = if normalized.is_empty() { vec![query.to_string()] } else { normalized };

		if let Some(key) = cache_key {
			let payload = ExpansionCachePayload { queries: result.clone() };
			let payload_json = match serde_json::to_value(&payload) {
				Ok(value) => value,
				Err(err) => {
					tracing::warn!(
						error = %err,
						cache_kind = CacheKind::Expansion.as_str(),
						cache_key_prefix = ranking::cache_key_prefix(&key),
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
						cache_key_prefix = ranking::cache_key_prefix(&key),
						hit = false,
						payload_size,
						ttl_days = cache_cfg.expansion_ttl_days,
						"Cache stored."
					);
				},
				Ok(None) => {
					tracing::warn!(
						cache_kind = CacheKind::Expansion.as_str(),
						cache_key_prefix = ranking::cache_key_prefix(&key),
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
						cache_key_prefix = ranking::cache_key_prefix(&key),
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
			.filter(|candidate| ranking::candidate_matches_note(&note_meta, candidate))
			.collect();
		let snippet_items = if filtered_candidates.is_empty() {
			Vec::new()
		} else {
			let pairs = ranking::collect_neighbor_pairs(&filtered_candidates);
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
				let snippet = ranking::stitch_snippet(
					candidate.note_id,
					chunk_row.chunk_index,
					&chunk_by_note_index,
				);

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
		let query_tokens = ranking::tokenize_query(query, MAX_MATCHED_TERMS);
		let scope_context_boost_by_scope =
			ranking::build_scope_context_boost_by_scope(&query_tokens, self.cfg.context.as_ref());
		let det_query_tokens = if self.cfg.ranking.deterministic.enabled
			&& self.cfg.ranking.deterministic.lexical.enabled
			&& self.cfg.ranking.deterministic.lexical.max_query_terms > 0
		{
			ranking::tokenize_query(
				query,
				self.cfg.ranking.deterministic.lexical.max_query_terms as usize,
			)
		} else {
			Vec::new()
		};
		let blend_policy = ranking::resolve_blend_policy(
			&self.cfg.ranking.blend,
			ranking_override.as_ref().and_then(|override_| override_.blend.as_ref()),
		)?;
		let diversity_policy = ranking::resolve_diversity_policy(
			&self.cfg.ranking.diversity,
			ranking_override.as_ref().and_then(|override_| override_.diversity.as_ref()),
		)?;
		let retrieval_sources_policy = ranking::resolve_retrieval_sources_policy(
			&self.cfg.ranking.retrieval_sources,
			ranking_override.as_ref().and_then(|override_| override_.retrieval_sources.as_ref()),
		)?;
		let policy_snapshot = ranking::build_policy_snapshot(
			&self.cfg,
			&blend_policy,
			&diversity_policy,
			&retrieval_sources_policy,
			ranking_override.as_ref(),
		);
		let policy_hash = ranking::hash_policy_snapshot(&policy_snapshot)?;
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

				match ranking::build_rerank_cache_key(
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
												cache_key_prefix = ranking::cache_key_prefix(&key),
												"Cache payload decode failed."
											);

											RerankCachePayload { items: Vec::new() }
										},
									};

								if let Some(scores) =
									ranking::build_cached_scores(&decoded, &cache_candidates)
								{
									tracing::info!(
										cache_kind = CacheKind::Rerank.as_str(),
										cache_key_prefix = ranking::cache_key_prefix(&key),
										hit = true,
										payload_size = payload.size_bytes,
										ttl_days = cache_cfg.rerank_ttl_days,
										"Cache hit."
									);
									cached_scores = Some(scores);
								} else {
									tracing::warn!(
										cache_kind = CacheKind::Rerank.as_str(),
										cache_key_prefix = ranking::cache_key_prefix(&key),
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
									cache_key_prefix = ranking::cache_key_prefix(&key),
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
									cache_key_prefix = ranking::cache_key_prefix(&key),
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
										cache_key_prefix = ranking::cache_key_prefix(key),
										hit = false,
										payload_size,
										ttl_days = cache_cfg.rerank_ttl_days,
										"Cache stored."
									);
								},
								Ok(None) => {
									tracing::warn!(
										cache_kind = CacheKind::Rerank.as_str(),
										cache_key_prefix = ranking::cache_key_prefix(key),
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
										cache_key_prefix = ranking::cache_key_prefix(key),
										"Cache write failed."
									);
								},
							}
						},
						Err(err) => {
							tracing::warn!(
								error = %err,
								cache_kind = CacheKind::Rerank.as_str(),
								cache_key_prefix = ranking::cache_key_prefix(key),
								"Cache payload encode failed."
							);
						},
					}
				}

				scores
			};

			scored = Vec::with_capacity(snippet_items.len());

			let rerank_ranks = ranking::build_rerank_ranks(&snippet_items, &scores);
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
					ranking::NormalizationKind::Rank =>
						ranking::rank_normalize(rerank_rank, total_rerank),
				};
				let retrieval_norm = match blend_policy.retrieval_normalization {
					ranking::NormalizationKind::Rank =>
						ranking::rank_normalize(retrieval_rank, total_retrieval),
				};
				let blend_retrieval_weight = if blend_policy.enabled {
					ranking::retrieval_weight_for_rank(retrieval_rank, &blend_policy.segments)
				} else {
					0.0
				};
				let retrieval_term = blend_retrieval_weight * retrieval_norm;
				let rerank_term = (1.0 - blend_retrieval_weight) * rerank_norm;
				let det_terms = ranking::compute_deterministic_ranking_terms(
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
		});

		let note_vectors = if diversity_policy.enabled {
			fetch_note_vectors_for_diversity(&self.db.pool, &results).await?
		} else {
			HashMap::new()
		};
		let (selected_results, diversity_decisions) =
			ranking::select_diverse_results(results, top_k, &diversity_policy, &note_vectors);

		ranking::attach_diversity_decisions_to_trace_candidates(
			&mut trace_candidates,
			&diversity_decisions,
		);

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
		let config_snapshot = ranking::build_config_snapshot(
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
			let (matched_terms, matched_fields) = ranking::match_terms_in_text(
				&query_tokens,
				&scored_chunk.item.snippet,
				scored_chunk.item.note.key.as_deref(),
				MAX_MATCHED_TERMS,
			);
			let matched_fields = ranking::merge_matched_fields(
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
						.map(ranking::build_diversity_explain)
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
						.map(ranking::build_diversity_explain)
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
	let blend_policy = ranking::resolve_blend_policy(
		&cfg.ranking.blend,
		ranking_override.and_then(|value| value.blend.as_ref()),
	)?;
	let diversity_policy = ranking::resolve_diversity_policy(
		&cfg.ranking.diversity,
		ranking_override.and_then(|value| value.diversity.as_ref()),
	)?;
	let retrieval_sources_policy = ranking::resolve_retrieval_sources_policy(
		&cfg.ranking.retrieval_sources,
		ranking_override.and_then(|value| value.retrieval_sources.as_ref()),
	)?;
	let snapshot = ranking::build_policy_snapshot(
		cfg,
		&blend_policy,
		&diversity_policy,
		&retrieval_sources_policy,
		ranking_override,
	);
	let hash = ranking::hash_policy_snapshot(&snapshot)?;
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

	let query_tokens = ranking::tokenize_query(trace.query.as_str(), MAX_MATCHED_TERMS);
	let scope_context_boost_by_scope =
		ranking::build_scope_context_boost_by_scope(&query_tokens, cfg.context.as_ref());
	let det_query_tokens = if cfg.ranking.deterministic.enabled
		&& cfg.ranking.deterministic.lexical.enabled
		&& cfg.ranking.deterministic.lexical.max_query_terms > 0
	{
		ranking::tokenize_query(
			trace.query.as_str(),
			cfg.ranking.deterministic.lexical.max_query_terms as usize,
		)
	} else {
		Vec::new()
	};
	let blend_policy = ranking::resolve_blend_policy(
		&cfg.ranking.blend,
		ranking_override.and_then(|override_| override_.blend.as_ref()),
	)?;
	let diversity_policy = ranking::resolve_diversity_policy(
		&cfg.ranking.diversity,
		ranking_override.and_then(|override_| override_.diversity.as_ref()),
	)?;
	let retrieval_sources_policy = ranking::resolve_retrieval_sources_policy(
		&cfg.ranking.retrieval_sources,
		ranking_override.and_then(|override_| override_.retrieval_sources.as_ref()),
	)?;
	let policy_snapshot = ranking::build_policy_snapshot(
		cfg,
		&blend_policy,
		&diversity_policy,
		&retrieval_sources_policy,
		ranking_override,
	);
	let policy_hash = ranking::hash_policy_snapshot(&policy_snapshot)?;
	let policy_id = format!("ranking_v2:{}", &policy_hash[..12.min(policy_hash.len())]);
	let now = trace.created_at;
	let total_rerank = u32::try_from(candidates.len()).unwrap_or(1).max(1);
	let total_retrieval = trace.candidate_count.max(1);
	let rerank_ranks = ranking::build_rerank_ranks_for_replay(candidates);
	let replay_diversity_decisions = ranking::extract_replay_diversity_decisions(candidates);
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
			ranking::NormalizationKind::Rank => ranking::rank_normalize(rerank_rank, total_rerank),
		};
		let retrieval_norm = match blend_policy.retrieval_normalization {
			ranking::NormalizationKind::Rank =>
				ranking::rank_normalize(retrieval_rank, total_retrieval),
		};
		let blend_retrieval_weight = if blend_policy.enabled {
			ranking::retrieval_weight_for_rank(retrieval_rank, &blend_policy.segments)
		} else {
			0.0
		};
		let retrieval_term = blend_retrieval_weight * retrieval_norm;
		let rerank_term = (1.0 - blend_retrieval_weight) * rerank_norm;
		let det_terms = ranking::compute_deterministic_ranking_terms(
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
				let ord = ranking::cmp_f32_desc(scored.final_score, existing.final_score);

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

	Ok(out)
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

	let query_hash = ranking::hash_query(query);
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
		let input = ranking::build_dense_embedding_input(
			"Find payments code.",
			Some("This is a billing API."),
		);

		assert!(input.starts_with("Find payments code.\n\nProject context:\n"));
		assert!(input.contains("This is a billing API."));
	}

	#[test]
	fn dense_embedding_input_skips_empty_project_context() {
		let input = ranking::build_dense_embedding_input("Find payments code.", Some("   "));

		assert_eq!(input, "Find payments code.");
	}

	#[test]
	fn scope_description_boost_matches_whole_tokens_only() {
		let tokens = vec!["go".to_string()];
		let boost = ranking::scope_description_boost(&tokens, "MongoDB operational notes.", 0.1);

		assert_eq!(boost, 0.0);
	}

	#[test]
	fn scope_description_boost_scales_by_fraction_of_matched_tokens() {
		let tokens = vec!["security".to_string(), "policy".to_string(), "deployment".to_string()];
		let boost = ranking::scope_description_boost(&tokens, "Security policy notes.", 0.12);

		assert!((boost - 0.08).abs() < 1e-4, "Unexpected boost: {boost}");
	}

	#[test]
	fn normalize_queries_includes_original_and_dedupes() {
		let queries = vec!["alpha".to_string(), "beta".to_string(), "alpha".to_string()];
		let normalized = ranking::normalize_queries(queries, "alpha", true, 4);

		assert_eq!(normalized, vec!["alpha".to_string(), "beta".to_string()]);
	}

	#[test]
	fn normalize_queries_respects_max_queries() {
		let queries =
			vec!["one".to_string(), "two".to_string(), "three".to_string(), "four".to_string()];
		let normalized = ranking::normalize_queries(queries, "zero", true, 3);

		assert_eq!(normalized.len(), 3);
	}

	#[test]
	fn dynamic_trigger_checks_candidates_and_score() {
		let cfg = SearchDynamic { min_candidates: 10, min_top_score: 0.2 };

		assert!(ranking::should_expand_dynamic(5, 0.9, &cfg));
		assert!(ranking::should_expand_dynamic(20, 0.1, &cfg));
		assert!(!ranking::should_expand_dynamic(20, 0.9, &cfg));
	}

	#[test]
	fn rank_normalize_maps_rank_to_unit_interval() {
		assert!((ranking::rank_normalize(1, 1) - 1.0).abs() < 1e-6);
		assert!((ranking::rank_normalize(1, 5) - 1.0).abs() < 1e-6);
		assert!((ranking::rank_normalize(3, 5) - 0.5).abs() < 1e-6);
		assert!((ranking::rank_normalize(5, 5) - 0.0).abs() < 1e-6);
		assert!((ranking::rank_normalize(0, 5) - 0.0).abs() < 1e-6);
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

	fn default_retrieval_sources_policy() -> ranking::ResolvedRetrievalSourcesPolicy {
		ranking::ResolvedRetrievalSourcesPolicy {
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
		let merged = ranking::merge_retrieval_candidates(
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
		let merged = ranking::merge_retrieval_candidates(
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
			ranking::BlendSegment { max_retrieval_rank: 3, retrieval_weight: 0.7 },
			ranking::BlendSegment { max_retrieval_rank: 10, retrieval_weight: 0.2 },
		];

		assert!((ranking::retrieval_weight_for_rank(1, &segments) - 0.7).abs() < 1e-6);
		assert!((ranking::retrieval_weight_for_rank(3, &segments) - 0.7).abs() < 1e-6);
		assert!((ranking::retrieval_weight_for_rank(4, &segments) - 0.2).abs() < 1e-6);
		assert!((ranking::retrieval_weight_for_rank(999, &segments) - 0.2).abs() < 1e-6);
	}

	#[test]
	fn blend_math_is_linear_and_additive() {
		let segments = vec![
			ranking::BlendSegment { max_retrieval_rank: 2, retrieval_weight: 0.7 },
			ranking::BlendSegment { max_retrieval_rank: 10, retrieval_weight: 0.2 },
		];
		let retrieval_rank = 3;
		let rerank_rank = 2;
		let retrieval_norm = ranking::rank_normalize(retrieval_rank, 10);
		let rerank_norm = ranking::rank_normalize(rerank_rank, 4);
		let blend_retrieval_weight = ranking::retrieval_weight_for_rank(retrieval_rank, &segments);

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
		let key_a = ranking::build_expansion_cache_key("alpha", 4, true, "llm", "model", 0.1_f32)
			.expect("Expected cache key.");
		let key_b = ranking::build_expansion_cache_key("alpha", 5, true, "llm", "model", 0.1_f32)
			.expect("Expected cache key.");

		assert_ne!(key_a, key_b);
	}

	#[test]
	fn rerank_cache_key_changes_with_updated_at() {
		let ts_a = OffsetDateTime::from_unix_timestamp(1).expect("Valid timestamp.");
		let ts_b = OffsetDateTime::from_unix_timestamp(2).expect("Valid timestamp.");
		let chunk_id = Uuid::new_v4();
		let key_a = ranking::build_rerank_cache_key("q", "rerank", "model", &[(chunk_id, ts_a)])
			.expect("Expected cache key.");
		let key_b = ranking::build_rerank_cache_key("q", "rerank", "model", &[(chunk_id, ts_b)])
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

		assert!(ranking::build_cached_scores(&payload, &candidates).is_none());
	}

	#[test]
	fn cache_key_prefix_is_stable() {
		let prefix = ranking::cache_key_prefix("abcd1234efgh5678");

		assert_eq!(prefix, "abcd1234efgh");
	}

	#[test]
	fn lexical_overlap_ratio_is_deterministic_and_bounded() {
		let query_tokens = vec!["deploy".to_string(), "steps".to_string()];
		let ratio = ranking::lexical_overlap_ratio(&query_tokens, "Deploy steps for staging.", 128);

		assert!((ratio - 1.0).abs() < 1e-6, "Unexpected ratio: {ratio}");

		let ratio = ranking::lexical_overlap_ratio(&query_tokens, "Deploy only.", 128);

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
		let terms = ranking::compute_deterministic_ranking_terms(
			&cfg,
			&ranking::tokenize_query(
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
		let terms = ranking::compute_deterministic_ranking_terms(
			&cfg,
			&ranking::tokenize_query(
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

		let policy = ranking::ResolvedDiversityPolicy {
			enabled: true,
			sim_threshold: 0.9,
			mmr_lambda: 0.7,
			max_skips: 64,
		};
		let (selected, decisions) =
			ranking::select_diverse_results(candidates, 2, &policy, &vectors);
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

		let policy = ranking::ResolvedDiversityPolicy {
			enabled: true,
			sim_threshold: 0.9,
			mmr_lambda: 0.7,
			max_skips: 0,
		};
		let (selected, decisions) =
			ranking::select_diverse_results(candidates, 2, &policy, &vectors);
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
		let decisions = ranking::extract_replay_diversity_decisions(&[first, second]);
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
