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
use serde::de::DeserializeOwned;
use sqlx::{PgExecutor, QueryBuilder};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::{ElfService, Error, Result};
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
const SEARCH_RANKING_EXPLAIN_SCHEMA_V1: &str = "search_ranking_explain/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpansionMode {
	Off,
	Always,
	Dynamic,
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RankingRequestOverride {
	#[serde(default)]
	pub blend: Option<BlendRankingOverride>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlendRankingOverride {
	pub enabled: Option<bool>,
	pub rerank_normalization: Option<String>,
	pub retrieval_normalization: Option<String>,
	pub segments: Option<Vec<BlendSegmentOverride>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlendSegmentOverride {
	pub max_retrieval_rank: u32,
	pub retrieval_weight: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchExplain {
	pub r#match: SearchMatchExplain,
	pub ranking: SearchRankingExplain,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchMatchExplain {
	pub matched_terms: Vec<String>,
	pub matched_fields: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchRankingExplain {
	pub schema: String,
	pub policy_id: String,
	#[serde(default)]
	pub signals: BTreeMap<String, serde_json::Value>,
	#[serde(default)]
	pub components: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchItem {
	pub result_handle: Uuid,
	pub note_id: Uuid,
	pub chunk_id: Uuid,
	pub chunk_index: i32,
	pub start_offset: i32,
	pub end_offset: i32,
	pub snippet: String,
	#[serde(rename = "type")]
	pub note_type: String,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResponse {
	pub trace_id: Uuid,
	pub items: Vec<SearchItem>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchExplainRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub result_handle: Uuid,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchExplainItem {
	pub result_handle: Uuid,
	pub note_id: Uuid,
	pub chunk_id: Option<Uuid>,
	pub rank: u32,
	pub explain: SearchExplain,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchExplainResponse {
	pub trace: SearchTrace,
	pub item: SearchExplainItem,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceGetRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub trace_id: Uuid,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceGetResponse {
	pub trace: SearchTrace,
	pub items: Vec<SearchExplainItem>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceReplayContext {
	pub trace_id: Uuid,
	pub query: String,
	pub candidate_count: u32,
	pub top_k: u32,
	#[serde(with = "crate::time_serde")]
	pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceReplayCandidate {
	pub note_id: Uuid,
	pub chunk_id: Uuid,
	pub retrieval_rank: u32,
	pub rerank_score: f32,
	pub note_scope: String,
	pub note_importance: f32,
	#[serde(with = "crate::time_serde")]
	pub note_updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceReplayItem {
	pub note_id: Uuid,
	pub chunk_id: Uuid,
	pub retrieval_rank: u32,
	pub final_score: f32,
	pub explain: SearchExplain,
}

#[derive(Debug, Clone)]
struct QueryEmbedding {
	text: String,
	vector: Vec<f32>,
}

#[derive(Debug, Clone)]
struct ChunkCandidate {
	chunk_id: Uuid,
	note_id: Uuid,
	chunk_index: i32,
	retrieval_rank: u32,
	updated_at: Option<OffsetDateTime>,
	embedding_version: Option<String>,
}

#[derive(Debug, Clone)]
struct RerankCacheCandidate {
	chunk_id: Uuid,
	updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ChunkRow {
	chunk_id: Uuid,
	note_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: String,
}

#[derive(Debug, Clone)]
struct ChunkMeta {
	chunk_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
}

#[derive(Debug, Clone)]
struct ChunkSnippet {
	note: NoteMeta,
	chunk: ChunkMeta,
	snippet: String,
	retrieval_rank: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ExpansionCachePayload {
	queries: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ExpansionOutput {
	queries: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RerankCacheItem {
	chunk_id: Uuid,
	updated_at: OffsetDateTime,
	score: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RerankCachePayload {
	items: Vec<RerankCacheItem>,
}

#[derive(Debug, Clone)]
struct CachePayload {
	value: serde_json::Value,
	size_bytes: usize,
}

#[derive(Debug)]
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
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TracePayload {
	trace: TraceRecord,
	items: Vec<TraceItemRecord>,
	#[serde(default)]
	candidates: Vec<TraceCandidateRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TraceItemRecord {
	item_id: Uuid,
	note_id: Uuid,
	chunk_id: Option<Uuid>,
	rank: u32,
	final_score: f32,
	explain: SearchExplain,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TraceCandidateRecord {
	candidate_id: Uuid,
	note_id: Uuid,
	chunk_id: Uuid,
	retrieval_rank: u32,
	rerank_score: f32,
	note_scope: String,
	note_importance: f32,
	note_updated_at: OffsetDateTime,
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
	top_k: u32,
	record_hits_enabled: bool,
	ranking_override: Option<RankingRequestOverride>,
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
					&[QueryEmbedding { text: query.clone(), vector: query_vec }],
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
						candidates,
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
			candidates,
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

				let Some(note) = note_meta.get(&candidate.note_id) else {
					continue;
				};
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
		let blend_policy = resolve_blend_policy(
			&self.cfg.ranking.blend,
			ranking_override.as_ref().and_then(|override_| override_.blend.as_ref()),
		)?;

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
				let final_score =
					retrieval_term + rerank_term + tie_breaker_score + scope_context_boost;

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
				});
			}
		}

		let mut best_by_note: HashMap<Uuid, ScoredChunk> = HashMap::new();

		let trace_candidates = if self.cfg.search.explain.capture_candidates {
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
						retrieval_rank: scored_chunk.item.retrieval_rank,
						rerank_score: scored_chunk.rerank_score,
						note_scope: note.scope.clone(),
						note_importance: note.importance,
						note_updated_at: note.updated_at,
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

		results
			.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap_or(Ordering::Equal));
		results.truncate(top_k as usize);

		if record_hits_enabled && !results.is_empty() {
			let mut tx = self.db.pool.begin().await?;
			record_hits(&mut *tx, query, &results, now).await?;
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

		let config_snapshot =
			build_config_snapshot(&self.cfg, &blend_policy, ranking_override.as_ref());

		let mut items = Vec::with_capacity(results.len());
		let mut trace_builder = SearchTraceBuilder::new(
			trace_context,
			config_snapshot,
			self.cfg.search.explain.retention_days,
			now,
		);

		for candidate in trace_candidates {
			trace_builder.push_candidate(candidate);
		}

		for (idx, scored_chunk) in results.into_iter().enumerate() {
			let rank = idx as u32 + 1;
			let (matched_terms, matched_fields) = match_terms_in_text(
				&query_tokens,
				&scored_chunk.item.snippet,
				scored_chunk.item.note.key.as_deref(),
				MAX_MATCHED_TERMS,
			);

			let mut signals = BTreeMap::new();

			signals.insert("blend.enabled".to_string(), serde_json::json!(blend_policy.enabled));
			signals.insert(
				"blend.retrieval_weight".to_string(),
				serde_json::json!(scored_chunk.blend_retrieval_weight),
			);
			signals.insert(
				"retrieval.rank".to_string(),
				serde_json::json!(scored_chunk.item.retrieval_rank),
			);
			signals.insert(
				"retrieval.norm".to_string(),
				serde_json::json!(scored_chunk.retrieval_norm),
			);
			signals
				.insert("rerank.score".to_string(), serde_json::json!(scored_chunk.rerank_score));
			signals.insert("rerank.rank".to_string(), serde_json::json!(scored_chunk.rerank_rank));
			signals.insert("rerank.norm".to_string(), serde_json::json!(scored_chunk.rerank_norm));
			signals.insert(
				"normalization.retrieval".to_string(),
				serde_json::json!(blend_policy.retrieval_normalization.as_str()),
			);
			signals.insert(
				"normalization.rerank".to_string(),
				serde_json::json!(blend_policy.rerank_normalization.as_str()),
			);
			signals.insert(
				"recency.tau_days".to_string(),
				serde_json::json!(self.cfg.ranking.recency_tau_days),
			);
			signals.insert(
				"tie_breaker.weight".to_string(),
				serde_json::json!(self.cfg.ranking.tie_breaker_weight),
			);
			signals.insert("age.days".to_string(), serde_json::json!(scored_chunk.age_days));
			signals.insert("importance".to_string(), serde_json::json!(scored_chunk.importance));
			signals.insert(
				"context.scope_boost".to_string(),
				serde_json::json!(scored_chunk.scope_context_boost),
			);

			let mut components = BTreeMap::new();

			components.insert("blend.retrieval".to_string(), scored_chunk.retrieval_term);
			components.insert("blend.rerank".to_string(), scored_chunk.rerank_term);
			components.insert("tie_breaker".to_string(), scored_chunk.tie_breaker_score);
			components.insert("context.scope_boost".to_string(), scored_chunk.scope_context_boost);

			let explain = SearchExplain {
				r#match: SearchMatchExplain {
					matched_terms: matched_terms.clone(),
					matched_fields: matched_fields.clone(),
				},
				ranking: SearchRankingExplain {
					schema: SEARCH_RANKING_EXPLAIN_SCHEMA_V1.to_string(),
					policy_id: "blend_v1".to_string(),
					signals,
					components,
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
				note_type: note.note_type.clone(),
				key: note.key.clone(),
				scope: note.scope.clone(),
				importance: note.importance,
				confidence: note.confidence,
				updated_at: note.updated_at,
				expires_at: note.expires_at,
				final_score: scored_chunk.final_score,
				source_ref: note.source_ref.clone(),
				explain: explain.clone(),
			});
			trace_builder.push_item(TraceItemRecord {
				item_id: result_handle,
				note_id: note.note_id,
				chunk_id: Some(chunk.chunk_id),
				rank,
				final_score: scored_chunk.final_score,
				explain,
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
	let snapshot = build_policy_snapshot(cfg, &blend_policy, ranking_override);
	let hash = hash_policy_snapshot(&snapshot)?;
	let prefix = &hash[..12.min(hash.len())];

	Ok(format!("blend_v1:{prefix}"))
}

pub fn replay_ranking_from_candidates(
	cfg: &Config,
	trace: &TraceReplayContext,
	ranking_override: Option<&RankingRequestOverride>,
	candidates: &[TraceReplayCandidate],
	top_k: u32,
) -> Result<Vec<TraceReplayItem>> {
	#[derive(Debug, Clone)]
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
	}

	let query_tokens = tokenize_query(trace.query.as_str(), MAX_MATCHED_TERMS);
	let scope_context_boost_by_scope =
		build_scope_context_boost_by_scope(&query_tokens, cfg.context.as_ref());
	let blend_policy = resolve_blend_policy(
		&cfg.ranking.blend,
		ranking_override.and_then(|override_| override_.blend.as_ref()),
	)?;
	let policy_snapshot = build_policy_snapshot(cfg, &blend_policy, ranking_override);
	let policy_hash = hash_policy_snapshot(&policy_snapshot)?;
	let policy_id = format!("blend_v1:{}", &policy_hash[..12.min(policy_hash.len())]);
	let now = trace.created_at;
	let total_rerank = u32::try_from(candidates.len()).unwrap_or(1).max(1);
	let total_retrieval = trace.candidate_count.max(1);
	let rerank_ranks = build_rerank_ranks_for_replay(candidates);
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
		let final_score = retrieval_term + rerank_term + tie_breaker_score + scope_context_boost;
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

	results.truncate(top_k.max(1) as usize);

	let mut out = Vec::with_capacity(results.len());

	for scored in results {
		let mut signals = BTreeMap::new();

		signals.insert("blend.enabled".to_string(), serde_json::json!(blend_policy.enabled));
		signals.insert(
			"blend.retrieval_weight".to_string(),
			serde_json::json!(scored.blend_retrieval_weight),
		);
		signals.insert("retrieval.rank".to_string(), serde_json::json!(scored.retrieval_rank));
		signals.insert("retrieval.norm".to_string(), serde_json::json!(scored.retrieval_norm));
		signals.insert("rerank.score".to_string(), serde_json::json!(scored.rerank_score));
		signals.insert("rerank.rank".to_string(), serde_json::json!(scored.rerank_rank));
		signals.insert("rerank.norm".to_string(), serde_json::json!(scored.rerank_norm));
		signals.insert(
			"normalization.retrieval".to_string(),
			serde_json::json!(blend_policy.retrieval_normalization.as_str()),
		);
		signals.insert(
			"normalization.rerank".to_string(),
			serde_json::json!(blend_policy.rerank_normalization.as_str()),
		);
		signals.insert(
			"recency.tau_days".to_string(),
			serde_json::json!(cfg.ranking.recency_tau_days),
		);
		signals.insert(
			"tie_breaker.weight".to_string(),
			serde_json::json!(cfg.ranking.tie_breaker_weight),
		);
		signals.insert("age.days".to_string(), serde_json::json!(scored.age_days));
		signals.insert("importance".to_string(), serde_json::json!(scored.importance));
		signals.insert(
			"context.scope_boost".to_string(),
			serde_json::json!(scored.scope_context_boost),
		);
		signals.insert("note.scope".to_string(), serde_json::json!(scored.note_scope));

		let mut components = BTreeMap::new();

		components.insert("blend.retrieval".to_string(), scored.retrieval_term);
		components.insert("blend.rerank".to_string(), scored.rerank_term);
		components.insert("tie_breaker".to_string(), scored.tie_breaker_score);
		components.insert("context.scope_boost".to_string(), scored.scope_context_boost);

		let explain = SearchExplain {
			r#match: SearchMatchExplain { matched_terms: Vec::new(), matched_fields: Vec::new() },
			ranking: SearchRankingExplain {
				schema: SEARCH_RANKING_EXPLAIN_SCHEMA_V1.to_string(),
				policy_id: policy_id.clone(),
				signals,
				components,
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

fn candidate_matches_note(note_meta: &HashMap<Uuid, NoteMeta>, candidate: &ChunkCandidate) -> bool {
	let Some(note) = note_meta.get(&candidate.note_id) else {
		return false;
	};

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
	let Some(description) = project_context_description else {
		return query.to_string();
	};
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
	let Some(context) = context else {
		return HashMap::new();
	};
	let Some(weight) = context.scope_boost_weight else {
		return HashMap::new();
	};

	if weight <= 0.0 || tokens.is_empty() {
		return HashMap::new();
	}

	let Some(descriptions) = context.scope_descriptions.as_ref() else {
		return HashMap::new();
	};

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

fn decode_json<T>(value: serde_json::Value, label: &str) -> Result<T>
where
	T: DeserializeOwned,
{
	serde_json::from_value(value)
		.map_err(|err| Error::Storage { message: format!("Invalid {label} value: {err}") })
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone)]
struct BlendSegment {
	max_retrieval_rank: u32,
	retrieval_weight: f32,
}

#[derive(Debug, Clone)]
struct ResolvedBlendPolicy {
	enabled: bool,
	rerank_normalization: NormalizationKind,
	retrieval_normalization: NormalizationKind,
	segments: Vec<BlendSegment>,
}

fn build_config_snapshot(
	cfg: &Config,
	blend_policy: &ResolvedBlendPolicy,
	ranking_override: Option<&RankingRequestOverride>,
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
			"recency_tau_days": cfg.ranking.recency_tau_days,
			"tie_breaker_weight": cfg.ranking.tie_breaker_weight,
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
	ranking_override: Option<&RankingRequestOverride>,
) -> serde_json::Value {
	let override_json = ranking_override.and_then(|value| serde_json::to_value(value).ok());

	serde_json::json!({
		"ranking": {
			"recency_tau_days": cfg.ranking.recency_tau_days,
			"tie_breaker_weight": cfg.ranking.tie_breaker_weight,
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

	sqlx::query(
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
	)
	.bind(trace_id)
	.bind(trace.tenant_id.as_str())
	.bind(trace.project_id.as_str())
	.bind(trace.agent_id.as_str())
	.bind(trace.read_profile.as_str())
	.bind(trace.query.as_str())
	.bind(trace.expansion_mode.as_str())
	.bind(expanded_queries_json)
	.bind(allowed_scopes_json)
	.bind(trace.candidate_count as i32)
	.bind(trace.top_k as i32)
	.bind(trace.config_snapshot)
	.bind(trace.trace_version)
	.bind(trace.created_at)
	.bind(trace.expires_at)
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
	retrieval_rank,
	rerank_score,
	note_scope,
	note_importance,
	note_updated_at,
	created_at,
	expires_at
) ",
		);
		builder.push_values(candidates, |mut b, candidate| {
			b.push_bind(candidate.candidate_id)
				.push_bind(trace_id)
				.push_bind(candidate.note_id)
				.push_bind(candidate.chunk_id)
				.push_bind(candidate.retrieval_rank as i32)
				.push_bind(candidate.rerank_score)
				.push_bind(candidate.note_scope)
				.push_bind(candidate.note_importance)
				.push_bind(candidate.note_updated_at)
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
	use elf_config::SearchDynamic;

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
}
