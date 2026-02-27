mod filter;
mod ranking;

pub use crate::ranking_explain_v2::{SearchRankingExplain, SearchRankingTerm};

use std::{
	cmp::Ordering,
	collections::{BTreeMap, HashMap, HashSet, VecDeque},
	slice,
};

use qdrant_client::qdrant::{
	Condition, Document, Filter, Fusion, MinShould, PrefetchQueryBuilder, Query,
	QueryPointsBuilder, ScoredPoint,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgConnection, PgExecutor, PgPool, QueryBuilder, Row};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{ElfService, Error, Result, access, ranking_explain_v2};
use elf_config::{Config, SearchCache};
use elf_storage::{
	models::MemoryNote,
	qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME},
};
use filter::{SearchFilter, SearchFilterImpact};
use ranking::{ResolvedBlendPolicy, ResolvedDiversityPolicy, ResolvedRetrievalSourcesPolicy};

const TRACE_VERSION: i32 = 3;
const MAX_MATCHED_TERMS: usize = 8;
const MAX_TRAJECTORY_STAGE_ITEMS: usize = 256;
const MAX_CANDIDATE_K: u32 = 1_024;
const QUERY_PLAN_SCHEMA: &str = "elf.search.query_plan";
const QUERY_PLAN_VERSION: &str = "v1";
const SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1: &str = "search_retrieval_trajectory/v1";
const SEARCH_FILTER_IMPACT_SCHEMA_V1: &str = "search_filter_impact/v1";
const RECENT_TRACES_SCHEMA_V1: &str = "elf.recent_traces/v1";
const TRACE_BUNDLE_SCHEMA_V1: &str = "elf.trace_bundle/v1";
const MAX_RECENT_TRACES_LIMIT: u32 = 200;
const DEFAULT_RECENT_TRACES_LIMIT: u32 = 50;
const DEFAULT_BOUNDED_STAGE_ITEMS_LIMIT: u32 = 64;
const DEFAULT_FULL_STAGE_ITEMS_LIMIT: u32 = 256;
const DEFAULT_BOUNDED_CANDIDATES_LIMIT: u32 = 0;
const DEFAULT_FULL_CANDIDATES_LIMIT: u32 = 200;
const MAX_TRACE_BUNDLE_ITEMS_LIMIT: u32 = 256;
const MAX_TRACE_BUNDLE_CANDIDATES_LIMIT: u32 = 1_000;
const RELATION_CONTEXT_SQL: &str = r#"
WITH selected_facts AS (
	SELECT DISTINCT ON (snc.selected_note_id, gf.fact_id)
		snc.selected_note_id,
		gf.fact_id,
		gf.scope,
		subject_entity.canonical AS subject_canonical,
		subject_entity.kind AS subject_kind,
		gf.predicate,
		gf.object_entity_id,
		object_entity.canonical AS object_canonical,
		object_entity.kind AS object_kind,
		gf.object_value,
		gf.valid_from,
		gf.valid_to
	FROM unnest($7::uuid[]) AS snc(selected_note_id)
	JOIN graph_fact_evidence gfe
		ON gfe.note_id = snc.selected_note_id
	JOIN graph_facts gf
		ON gf.fact_id = gfe.fact_id
	JOIN graph_entities subject_entity
		ON subject_entity.entity_id = gf.subject_entity_id
		AND subject_entity.tenant_id = $1
		AND subject_entity.project_id = $2
	LEFT JOIN graph_entities object_entity
		ON object_entity.entity_id = gf.object_entity_id
		AND object_entity.tenant_id = $1
		AND object_entity.project_id = $2
	WHERE gf.tenant_id = $1
		AND gf.project_id = $2
		AND (
			($5 AND gf.scope = 'agent_private' AND gf.agent_id = $3)
			OR gf.scope = ANY($6::text[])
		)
		AND gf.valid_from <= $4
		AND (gf.valid_to IS NULL OR gf.valid_to > $4)
	ORDER BY snc.selected_note_id, gf.fact_id, gf.valid_from DESC, gf.fact_id ASC
),
ranked_facts AS (
	SELECT
		selected_note_id,
		fact_id,
		scope,
		subject_canonical,
		subject_kind,
		predicate,
		object_entity_id,
		object_canonical,
		object_kind,
		object_value,
		valid_from,
		valid_to,
		ROW_NUMBER() OVER (
			PARTITION BY selected_note_id
			ORDER BY valid_from DESC, fact_id ASC
		) AS fact_rank
	FROM selected_facts
),
bounded_facts AS (
	SELECT
		selected_note_id,
		fact_id,
		scope,
		subject_canonical,
		subject_kind,
		predicate,
		object_entity_id,
		object_canonical,
		object_kind,
		object_value,
		valid_from,
		valid_to,
		fact_rank
	FROM ranked_facts
	WHERE fact_rank <= $9
),
evidence_ranked AS (
	SELECT
		bf.selected_note_id,
		bf.fact_id,
		bf.scope,
		bf.subject_canonical,
		bf.subject_kind,
		bf.predicate,
		bf.object_entity_id,
		bf.object_canonical,
		bf.object_kind,
		bf.object_value,
		bf.valid_from,
		bf.valid_to,
		bf.fact_rank,
		e.note_id AS evidence_note_id,
		e.created_at AS evidence_created_at,
		ROW_NUMBER() OVER (
			PARTITION BY bf.selected_note_id, bf.fact_id
			ORDER BY e.created_at ASC, e.note_id ASC
		) AS evidence_rank
	FROM bounded_facts bf
	JOIN graph_fact_evidence e
		ON e.fact_id = bf.fact_id
),
fact_contexts AS (
	SELECT
		selected_note_id,
		fact_id,
		scope,
		subject_canonical,
		subject_kind,
		predicate,
		object_entity_id,
		object_canonical,
		object_kind,
		object_value,
		valid_from,
		valid_to,
		fact_rank,
		ARRAY_AGG(evidence_note_id ORDER BY evidence_created_at ASC, evidence_note_id ASC) AS evidence_note_ids
	FROM evidence_ranked
	WHERE evidence_rank <= $8
	GROUP BY
		selected_note_id,
		fact_id,
		scope,
		subject_canonical,
		subject_kind,
		predicate,
		object_entity_id,
		object_canonical,
		object_kind,
		object_value,
		valid_from,
		valid_to,
		fact_rank
)
SELECT
	selected_note_id AS note_id,
	fact_id,
	scope,
	subject_canonical,
	subject_kind,
	predicate,
	object_entity_id,
	object_canonical,
	object_kind,
	object_value,
	valid_from,
	valid_to,
	evidence_note_ids
FROM fact_contexts
ORDER BY note_id, fact_rank
"#;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub token_id: Option<String>,
	#[serde(default)]
	pub payload_level: PayloadLevel,
	pub read_profile: String,
	pub query: String,
	pub top_k: Option<u32>,
	pub candidate_k: Option<u32>,

	pub filter: Option<Value>,
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
	pub recursive_weight: Option<f32>,
	pub recursive_priority: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchExplain {
	pub r#match: SearchMatchExplain,
	pub ranking: SearchRankingExplain,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relation_context: Option<Vec<SearchExplainRelationContext>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub diversity: Option<SearchDiversityExplain>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchExplainRelationContext {
	pub fact_id: Uuid,
	pub scope: String,
	pub subject: SearchExplainRelationEntityRef,
	pub predicate: String,
	pub object: SearchExplainRelationContextObject,
	#[serde(with = "crate::time_serde")]
	pub valid_from: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	pub valid_to: Option<OffsetDateTime>,
	#[serde(default)]
	pub evidence_note_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchExplainRelationEntityRef {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub canonical: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub kind: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchExplainRelationContextObject {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub entity: Option<SearchExplainRelationEntityRef>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub value: Option<String>,
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
	pub source_ref: Value,
	pub explain: SearchExplain,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResponse {
	pub trace_id: Uuid,
	pub items: Vec<SearchItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchRawPlannedResponse {
	pub trace_id: Uuid,
	pub items: Vec<SearchItem>,
	pub query_plan: QueryPlan,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryPlan {
	pub schema: String,
	pub version: String,
	pub stages: Vec<QueryPlanStage>,
	pub intent: QueryPlanIntent,
	pub rewrite: QueryPlanRewrite,
	pub retrieval_stages: Vec<QueryPlanRetrievalStage>,
	pub fusion_policy: QueryPlanFusionPolicy,
	pub rerank_policy: QueryPlanRerankPolicy,
	pub budget: QueryPlanBudget,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryPlanStage {
	pub name: String,
	pub details: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryPlanIntent {
	pub query: String,
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub read_profile: String,
	pub allowed_scopes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryPlanRewrite {
	pub expansion_mode: String,
	pub expanded_queries: Vec<String>,
	pub dynamic_gate: QueryPlanDynamicGate,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryPlanDynamicGate {
	pub considered: bool,
	pub should_expand: Option<bool>,
	pub observed_candidates: Option<u32>,
	pub observed_top_score: Option<f32>,
	pub min_candidates: u32,
	pub min_top_score: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryPlanRetrievalStage {
	pub name: String,
	pub source: String,
	pub enabled: bool,
	pub candidate_limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryPlanFusionPolicy {
	pub strategy: String,
	pub fusion_weight: f32,
	pub structured_field_weight: f32,
	pub recursive_weight: f32,
	pub fusion_priority: u32,
	pub structured_field_priority: u32,
	pub recursive_priority: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryPlanBlendSegment {
	pub max_retrieval_rank: u32,
	pub retrieval_weight: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryPlanRerankPolicy {
	pub provider_id: String,
	pub model: String,
	pub blend_enabled: bool,
	pub rerank_normalization: String,
	pub retrieval_normalization: String,
	pub blend_segments: Vec<QueryPlanBlendSegment>,
	pub diversity_enabled: bool,
	pub diversity_sim_threshold: f32,
	pub diversity_mmr_lambda: f32,
	pub diversity_max_skips: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryPlanBudget {
	pub top_k: u32,
	pub candidate_k: u32,
	pub prefilter_max_candidates: u32,
	pub expansion_max_queries: u32,
	pub cache_enabled: bool,
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
	pub config_snapshot: Value,
	#[serde(with = "crate::time_serde")]
	pub created_at: OffsetDateTime,
	pub trace_version: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchTrajectorySummary {
	pub schema: String,
	pub stages: Vec<SearchTrajectorySummaryStage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchTrajectorySummaryStage {
	pub stage_order: u32,
	pub stage_name: String,
	pub item_count: u32,
	pub stats: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchTrajectoryStage {
	pub stage_order: u32,
	pub stage_name: String,
	pub stage_payload: Value,
	pub items: Vec<SearchTrajectoryStageItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchTrajectoryStageItem {
	pub item_id: Option<Uuid>,
	pub note_id: Option<Uuid>,
	pub chunk_id: Option<Uuid>,
	pub metrics: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchTrajectoryResponse {
	pub trace: SearchTrace,
	pub trajectory: SearchTrajectorySummary,
	pub stages: Vec<SearchTrajectoryStage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchExplainTrajectory {
	pub schema: String,
	pub stages: Vec<SearchExplainTrajectoryStage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchExplainTrajectoryStage {
	pub stage_order: u32,
	pub stage_name: String,
	pub metrics: Value,
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
	#[serde(skip_serializing_if = "Option::is_none")]
	pub trajectory: Option<SearchExplainTrajectory>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceRecentListRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,

	pub limit: Option<u32>,

	pub cursor_created_at: Option<OffsetDateTime>,

	pub cursor_trace_id: Option<Uuid>,

	pub agent_id_filter: Option<String>,

	pub read_profile: Option<String>,
	#[serde(with = "crate::time_serde::option")]
	pub created_after: Option<OffsetDateTime>,
	#[serde(with = "crate::time_serde::option")]
	pub created_before: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecentTraceHeader {
	pub trace_id: Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub read_profile: String,
	pub query: String,
	#[serde(with = "crate::time_serde")]
	pub created_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceRecentCursor {
	#[serde(with = "crate::time_serde")]
	pub created_at: OffsetDateTime,
	pub trace_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceRecentListResponse {
	pub schema: String,
	pub traces: Vec<RecentTraceHeader>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub next_cursor: Option<TraceRecentCursor>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TraceBundleMode {
	#[default]
	Bounded,
	Full,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceBundleGetRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub trace_id: Uuid,
	#[serde(default)]
	pub mode: TraceBundleMode,

	pub stage_items_limit: Option<u32>,

	pub candidates_limit: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceBundleResponse {
	pub schema: String,
	#[serde(with = "crate::time_serde")]
	pub generated_at: OffsetDateTime,
	pub trace: SearchTrace,
	pub items: Vec<SearchExplainItem>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub trajectory_summary: Option<SearchTrajectorySummary>,
	pub stages: Vec<SearchTrajectoryStage>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub candidates: Option<Vec<TraceReplayCandidate>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceGetRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub trace_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceTrajectoryGetRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub trace_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceGetResponse {
	pub trace: SearchTrace,
	pub items: Vec<SearchExplainItem>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub trajectory_summary: Option<SearchTrajectorySummary>,
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

struct ScoreSnippetArgs<'a, 'k> {
	query: &'a str,
	snippet_items: Vec<ChunkSnippet>,
	scope_context_boost_by_scope: &'a HashMap<&'k str, f32>,
	det_query_tokens: &'a [String],
	blend_policy: &'a ResolvedBlendPolicy,
	cache_cfg: &'a SearchCache,
	now: OffsetDateTime,
	candidate_count: usize,
}

struct ScoreCandidateCtx<'a, 'k> {
	cfg: &'a Config,
	blend_policy: &'a ResolvedBlendPolicy,
	scope_context_boost_by_scope: &'a HashMap<&'k str, f32>,
	det_query_tokens: &'a [String],
	now: OffsetDateTime,
	total_rerank: u32,
	total_retrieval: u32,
}

struct MaybeDynamicSearchArgs<'a> {
	path: RawSearchPath,
	enabled: bool,
	trace_id: Uuid,
	query: &'a str,
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	token_id: Option<&'a str>,
	read_profile: &'a str,
	allowed_scopes: &'a [String],
	project_context_description: Option<&'a str>,
	filter: &'a Filter,
	service_filter: Option<&'a SearchFilter>,
	candidate_k: u32,
	requested_candidate_k: u32,
	effective_candidate_k: u32,
	top_k: u32,
	record_hits_enabled: bool,
	ranking_override: Option<&'a RankingRequestOverride>,
	retrieval_sources_policy: &'a ResolvedRetrievalSourcesPolicy,
}

struct SearchRetrievalArgs<'a> {
	query: &'a str,
	expansion_mode: ExpansionMode,
	project_context_description: Option<&'a str>,
	filter: &'a Filter,
	candidate_k: u32,
	baseline_vector: Option<&'a Vec<f32>>,
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	allowed_scopes: &'a [String],
	retrieval_sources_policy: &'a ResolvedRetrievalSourcesPolicy,
}

struct RecursiveRetrievalArgs<'a> {
	query: &'a str,
	query_vec: &'a [f32],
	filter: &'a Filter,
	candidate_k: u32,
	retrieval_sources_policy: &'a ResolvedRetrievalSourcesPolicy,
	seed_candidates: &'a [ChunkCandidate],
}

struct SearchRetrievalResult {
	expanded_queries: Vec<String>,
	candidates: Vec<ChunkCandidate>,
	structured_matches: HashMap<Uuid, Vec<String>>,
	recursive: Option<RecursiveRetrievalResult>,
}

#[derive(Debug, Default, Clone)]
struct RecursiveRetrievalResult {
	enabled: bool,
	rounds_executed: u32,
	scopes_seeded: usize,
	scopes_queried: usize,
	candidates_before: usize,
	candidates_after: usize,
	candidates_added: usize,
	total_queries: u32,
	stop_reason: Option<String>,
	candidates: Vec<ChunkCandidate>,
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
	scope: Option<String>,
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
	agent_id: String,
	importance: f32,
	confidence: f32,
	updated_at: OffsetDateTime,
	expires_at: Option<OffsetDateTime>,
	source_ref: Value,
	embedding_version: String,
	hit_count: i64,
	last_hit_at: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, FromRow)]
struct ChunkRow {
	chunk_id: Uuid,
	note_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: String,
}

#[derive(Clone, Debug, FromRow)]
struct NoteVectorRow {
	note_id: Uuid,
	vec_text: String,
}

#[derive(Clone, Debug, FromRow)]
struct SearchExplainTraceRow {
	trace_id: Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
	query: String,
	expansion_mode: String,
	expanded_queries: Value,
	allowed_scopes: Value,
	candidate_count: i32,
	top_k: i32,
	config_snapshot: Value,
	trace_version: i32,
	created_at: OffsetDateTime,
	item_id: Uuid,
	note_id: Uuid,
	chunk_id: Option<Uuid>,
	rank: i32,
	explain: Value,
}

#[derive(Clone, Debug, FromRow)]
struct SearchRelationContextRow {
	note_id: Uuid,
	fact_id: Uuid,
	scope: String,
	subject_canonical: Option<String>,
	subject_kind: Option<String>,
	predicate: String,
	object_entity_id: Option<Uuid>,
	object_canonical: Option<String>,
	object_kind: Option<String>,
	object_value: Option<String>,
	valid_from: OffsetDateTime,
	valid_to: Option<OffsetDateTime>,
	evidence_note_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, FromRow)]
struct SearchTraceRow {
	trace_id: Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
	query: String,
	expansion_mode: String,
	expanded_queries: Value,
	allowed_scopes: Value,
	candidate_count: i32,
	top_k: i32,
	config_snapshot: Value,
	trace_version: i32,
	created_at: OffsetDateTime,
}

#[derive(Clone, Debug, FromRow)]
struct SearchTraceItemRow {
	item_id: Uuid,
	note_id: Uuid,
	chunk_id: Option<Uuid>,
	rank: i32,
	explain: Value,
}

#[derive(Clone, Debug, FromRow)]
struct SearchRecentTraceRow {
	trace_id: Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
	query: String,
	created_at: OffsetDateTime,
}

#[derive(Clone, Debug, FromRow)]
struct TraceCandidateSnapshotRow {
	candidate_snapshot: Value,
}

#[derive(Clone, Debug, FromRow)]
struct StructuredFieldHitRow {
	note_id: Uuid,
	field_kind: String,
}

#[derive(Clone, Debug, FromRow)]
struct BestChunkForNoteRow {
	note_id: Uuid,
	chunk_id: Uuid,
	chunk_index: i32,
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
	value: Value,
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
	#[serde(default)]
	stages: Vec<TraceTrajectoryStageRecord>,
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
	config_snapshot: Value,
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
	candidate_snapshot: Value,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TraceTrajectoryStageRecord {
	stage_id: Uuid,
	stage_order: u32,
	stage_name: String,
	stage_payload: Value,
	created_at: OffsetDateTime,
	#[serde(default)]
	items: Vec<TraceTrajectoryStageItemRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TraceTrajectoryStageItemRecord {
	id: Uuid,
	item_id: Option<Uuid>,
	note_id: Option<Uuid>,
	chunk_id: Option<Uuid>,
	metrics: Value,
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
	stages: Vec<TraceTrajectoryStageRecord>,
}
impl SearchTraceBuilder {
	fn new(
		context: TraceContext<'_>,
		config_snapshot: Value,
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

		Self { trace, items: Vec::new(), candidates: Vec::new(), stages: Vec::new() }
	}

	fn push_item(&mut self, item: TraceItemRecord) {
		self.items.push(item);
	}

	fn push_candidate(&mut self, candidate: TraceCandidateRecord) {
		self.candidates.push(candidate);
	}

	fn push_stage(&mut self, stage: TraceTrajectoryStageRecord) {
		self.stages.push(stage);
	}

	fn build(self) -> TracePayload {
		TracePayload {
			trace: self.trace,
			items: self.items,
			candidates: self.candidates,
			stages: self.stages,
		}
	}
}

struct FinishSearchArgs<'a> {
	path: RawSearchPath,
	trace_id: Uuid,
	query: &'a str,
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	token_id: Option<&'a str>,
	read_profile: &'a str,
	allowed_scopes: &'a [String],
	expanded_queries: Vec<String>,
	expansion_mode: ExpansionMode,
	candidates: Vec<ChunkCandidate>,
	structured_matches: HashMap<Uuid, Vec<String>>,
	recursive_retrieval: Option<RecursiveRetrievalResult>,
	top_k: u32,
	record_hits_enabled: bool,
	ranking_override: Option<RankingRequestOverride>,
	filter: Option<&'a SearchFilter>,
	requested_candidate_k: u32,
	effective_candidate_k: u32,
}

struct FinishSearchPolicies {
	blend_policy: ResolvedBlendPolicy,
	diversity_policy: ResolvedDiversityPolicy,
	retrieval_sources_policy: ResolvedRetrievalSourcesPolicy,
	policy_snapshot: Value,
	policy_id: String,
}

struct FinishSearchScoringResult {
	query_tokens: Vec<String>,
	filtered_candidates: Vec<ChunkCandidate>,
	scored_count: usize,
	snippet_count: usize,
	filtered_candidate_count: usize,
	filter_impact: Option<SearchFilterImpact>,
	trace_candidates: Vec<TraceCandidateRecord>,
	fused_results: Vec<ScoredChunk>,
	selected_results: Vec<ScoredChunk>,
	diversity_decisions: HashMap<Uuid, DiversityDecision>,
	selected_count: usize,
}

struct BuildTraceArgs<'a> {
	path: RawSearchPath,
	trace_id: Uuid,
	query: &'a str,
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	token_id: Option<&'a str>,
	read_profile: &'a str,
	expansion_mode: ExpansionMode,
	expanded_queries: Vec<String>,
	allowed_scopes: &'a [String],
	candidate_count: usize,
	filtered_candidate_count: usize,
	snippet_count: usize,
	scored_count: usize,
	fused_count: usize,
	selected_count: usize,
	top_k: u32,
	query_tokens: &'a [String],
	structured_matches: &'a HashMap<Uuid, Vec<String>>,
	recursive_retrieval: Option<&'a RecursiveRetrievalResult>,
	policies: &'a FinishSearchPolicies,
	diversity_decisions: &'a HashMap<Uuid, DiversityDecision>,
	recall_candidates: Vec<ChunkCandidate>,
	fused_results: Vec<ScoredChunk>,
	selected_results: Vec<ScoredChunk>,
	relation_contexts: HashMap<Uuid, Vec<SearchExplainRelationContext>>,
	trace_candidates: Vec<TraceCandidateRecord>,
	now: OffsetDateTime,
	ranking_override: &'a Option<RankingRequestOverride>,
	filter_impact: Option<SearchFilterImpact>,
}

struct BuildQueryPlanArgs<'a> {
	path: RawSearchPath,
	query: &'a str,
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	read_profile: &'a str,
	allowed_scopes: &'a [String],
	expansion_mode: ExpansionMode,
	expanded_queries: Vec<String>,
	top_k: u32,
	candidate_k: u32,
	retrieval_sources_policy: &'a ResolvedRetrievalSourcesPolicy,
	recursive_enabled: bool,
	policies: &'a FinishSearchPolicies,
	dynamic_gate: DynamicGateSummary,
}

struct RawSearchExecutionContext {
	tenant_id: String,
	project_id: String,
	agent_id: String,
	token_id: Option<String>,
	top_k: u32,
	candidate_k: u32,
	requested_candidate_k: u32,
	effective_candidate_k: u32,
	query: String,
	read_profile: String,
	filter: Option<SearchFilter>,
	record_hits_enabled: bool,
	ranking_override: Option<RankingRequestOverride>,
	retrieval_sources_policy: ResolvedRetrievalSourcesPolicy,
	expansion_mode: ExpansionMode,
	trace_id: Uuid,
	project_context_description: Option<String>,
	allowed_scopes: Vec<String>,
	policies: FinishSearchPolicies,
}

struct QueryPlanStagesArgs<'a> {
	path: RawSearchPath,
	query: &'a str,
	read_profile: &'a str,
	allowed_scope_count: usize,
	rewrite: &'a QueryPlanRewrite,
	retrieval_stages: &'a [QueryPlanRetrievalStage],
	fusion_policy: &'a QueryPlanFusionPolicy,
	rerank_policy: &'a QueryPlanRerankPolicy,
	budget: &'a QueryPlanBudget,
}

struct BuildSearchItemArgs<'a> {
	cfg: &'a Config,
	policy_id: &'a str,
	blend_policy: &'a ResolvedBlendPolicy,
	diversity_policy: &'a ResolvedDiversityPolicy,
	diversity_decisions: &'a HashMap<Uuid, DiversityDecision>,
	query_tokens: &'a [String],
	structured_matches: &'a HashMap<Uuid, Vec<String>>,
	relation_contexts: &'a HashMap<Uuid, Vec<SearchExplainRelationContext>>,
	scored_chunk: ScoredChunk,
	rank: u32,
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

#[derive(Debug)]
struct FieldHit {
	note_id: Uuid,
	field_kind: String,
}

struct StructuredFieldHitArgs<'a> {
	embed_version: &'a str,
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	now: OffsetDateTime,
	vec_text: &'a str,
	retrieval_limit: i64,
	private_allowed: bool,
	non_private_scopes: &'a [String],
}

#[derive(Clone, Debug)]
struct StructuredFieldRetrievalResult {
	candidates: Vec<ChunkCandidate>,
	structured_matches: HashMap<Uuid, Vec<String>>,
}

#[derive(Debug, Clone)]
struct RetrievalSourceCandidates {
	source: RetrievalSourceKind,
	candidates: Vec<ChunkCandidate>,
}

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

#[derive(Clone, Debug, Default)]
struct DynamicGateSummary {
	considered: bool,
	should_expand: Option<bool>,
	observed_candidates: Option<u32>,
	observed_top_score: Option<f32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PayloadLevel {
	#[default]
	L0,
	L1,
	L2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExpansionMode {
	Off,
	Always,
	Dynamic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RawSearchPath {
	Quick,
	Planned,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RetrievalSourceKind {
	Fusion,
	StructuredField,
	Recursive,
}

impl ElfService {
	pub async fn search_raw_quick(&self, req: SearchRequest) -> Result<SearchResponse> {
		self.execute_search_raw_path(req, RawSearchPath::Quick)
			.await
			.map(|response| SearchResponse { trace_id: response.trace_id, items: response.items })
	}

	pub async fn search_raw_planned(&self, req: SearchRequest) -> Result<SearchRawPlannedResponse> {
		self.execute_search_raw_path(req, RawSearchPath::Planned).await
	}

	pub async fn search_raw(&self, req: SearchRequest) -> Result<SearchResponse> {
		self.search_raw_planned(req)
			.await
			.map(|response| SearchResponse { trace_id: response.trace_id, items: response.items })
	}

	async fn execute_search_raw_path(
		&self,
		req: SearchRequest,
		path: RawSearchPath,
	) -> Result<SearchRawPlannedResponse> {
		let context = self.prepare_raw_search_execution(req, path)?;

		if context.allowed_scopes.is_empty() {
			return self.execute_search_raw_no_allowed_scopes(&context, path).await;
		}

		let dynamic_gate_enabled =
			path == RawSearchPath::Planned && context.expansion_mode == ExpansionMode::Dynamic;

		self.execute_search_raw_with_allowed_scopes(&context, path, dynamic_gate_enabled).await
	}

	async fn execute_search_raw_no_allowed_scopes(
		&self,
		context: &RawSearchExecutionContext,
		path: RawSearchPath,
	) -> Result<SearchRawPlannedResponse> {
		let expanded_queries = vec![context.query.clone()];
		let response = self
			.finish_search(FinishSearchArgs {
				path,
				trace_id: context.trace_id,
				query: context.query.as_str(),
				tenant_id: context.tenant_id.as_str(),
				project_id: context.project_id.as_str(),
				agent_id: context.agent_id.as_str(),
				token_id: context.token_id.as_deref(),
				read_profile: context.read_profile.as_str(),
				allowed_scopes: &context.allowed_scopes,
				expanded_queries: expanded_queries.clone(),
				expansion_mode: context.expansion_mode,
				candidates: Vec::new(),
				structured_matches: HashMap::new(),
				recursive_retrieval: None,
				top_k: context.top_k,
				record_hits_enabled: context.record_hits_enabled,
				ranking_override: context.ranking_override.clone(),
				filter: context.filter.as_ref(),
				requested_candidate_k: context.requested_candidate_k,
				effective_candidate_k: context.effective_candidate_k,
			})
			.await?;

		Ok(self.build_raw_planned_response(
			context,
			path,
			response,
			expanded_queries,
			DynamicGateSummary::default(),
		))
	}

	async fn execute_search_raw_with_allowed_scopes(
		&self,
		context: &RawSearchExecutionContext,
		path: RawSearchPath,
		dynamic_gate_enabled: bool,
	) -> Result<SearchRawPlannedResponse> {
		let filter = build_search_filter(
			context.tenant_id.as_str(),
			context.project_id.as_str(),
			context.agent_id.as_str(),
			&context.allowed_scopes,
		);
		let retrieval_candidate_k = if context.filter.is_some() {
			context.effective_candidate_k
		} else {
			context.candidate_k
		};
		let (baseline_vector, early_response, dynamic_gate) = self
			.maybe_finish_dynamic_search(MaybeDynamicSearchArgs {
				path,
				enabled: dynamic_gate_enabled,
				trace_id: context.trace_id,
				query: context.query.as_str(),
				tenant_id: context.tenant_id.as_str(),
				project_id: context.project_id.as_str(),
				agent_id: context.agent_id.as_str(),
				token_id: context.token_id.as_deref(),
				read_profile: context.read_profile.as_str(),
				allowed_scopes: &context.allowed_scopes,
				project_context_description: context.project_context_description.as_deref(),
				filter: &filter,
				service_filter: context.filter.as_ref(),
				candidate_k: retrieval_candidate_k,
				requested_candidate_k: context.requested_candidate_k,
				effective_candidate_k: context.effective_candidate_k,
				top_k: context.top_k,
				record_hits_enabled: context.record_hits_enabled,
				ranking_override: context.ranking_override.as_ref(),
				retrieval_sources_policy: &context.retrieval_sources_policy,
			})
			.await?;

		if let Some(response) = early_response {
			return Ok(self.build_raw_planned_response(
				context,
				path,
				response,
				vec![context.query.clone()],
				dynamic_gate,
			));
		}

		let retrieval = self
			.retrieve_search_candidates(SearchRetrievalArgs {
				query: context.query.as_str(),
				expansion_mode: context.expansion_mode,
				project_context_description: context.project_context_description.as_deref(),
				filter: &filter,
				candidate_k: retrieval_candidate_k,
				baseline_vector: baseline_vector.as_ref(),
				tenant_id: context.tenant_id.as_str(),
				project_id: context.project_id.as_str(),
				agent_id: context.agent_id.as_str(),
				allowed_scopes: &context.allowed_scopes,
				retrieval_sources_policy: &context.retrieval_sources_policy,
			})
			.await?;
		let expanded_queries = retrieval.expanded_queries.clone();
		let response = self
			.finish_search(FinishSearchArgs {
				path,
				trace_id: context.trace_id,
				query: context.query.as_str(),
				tenant_id: context.tenant_id.as_str(),
				project_id: context.project_id.as_str(),
				agent_id: context.agent_id.as_str(),
				token_id: context.token_id.as_deref(),
				read_profile: context.read_profile.as_str(),
				allowed_scopes: &context.allowed_scopes,
				expanded_queries: retrieval.expanded_queries,
				expansion_mode: context.expansion_mode,
				candidates: retrieval.candidates,
				structured_matches: retrieval.structured_matches,
				recursive_retrieval: retrieval.recursive,
				top_k: context.top_k,
				record_hits_enabled: context.record_hits_enabled,
				ranking_override: context.ranking_override.clone(),
				filter: context.filter.as_ref(),
				requested_candidate_k: context.requested_candidate_k,
				effective_candidate_k: context.effective_candidate_k,
			})
			.await?;

		Ok(self.build_raw_planned_response(context, path, response, expanded_queries, dynamic_gate))
	}

	fn prepare_raw_search_execution(
		&self,
		req: SearchRequest,
		path: RawSearchPath,
	) -> Result<RawSearchExecutionContext> {
		let tenant_id = req.tenant_id.trim().to_string();
		let project_id = req.project_id.trim().to_string();
		let agent_id = req.agent_id.trim().to_string();
		let token_id = req
			.token_id
			.as_deref()
			.map(str::trim)
			.filter(|value| !value.is_empty())
			.map(|value| value.to_string());

		validate_search_request_inputs(
			tenant_id.as_str(),
			project_id.as_str(),
			agent_id.as_str(),
			req.query.as_str(),
		)?;

		let top_k = req.top_k.unwrap_or(self.cfg.memory.top_k).max(1);
		let candidate_k = req.candidate_k.unwrap_or(self.cfg.memory.candidate_k).max(top_k);
		let requested_candidate_k = candidate_k;
		let filter = req
			.filter
			.as_ref()
			.map(SearchFilter::parse)
			.transpose()
			.map_err(|err| Error::InvalidRequest { message: err.to_string() })?;
		let effective_candidate_k = if filter.is_some() {
			requested_candidate_k.saturating_mul(3).min(MAX_CANDIDATE_K).max(top_k)
		} else {
			requested_candidate_k
		};
		let query = req.query;
		let read_profile = req.read_profile;
		let record_hits_enabled = req.record_hits.unwrap_or(false);
		let ranking_override = req.ranking;
		let retrieval_sources_policy = ranking::resolve_retrieval_sources_policy(
			&self.cfg.ranking.retrieval_sources,
			ranking_override.as_ref().and_then(|override_| override_.retrieval_sources.as_ref()),
		)?;
		let expansion_mode = match path {
			RawSearchPath::Quick => ExpansionMode::Off,
			RawSearchPath::Planned => ranking::resolve_expansion_mode(&self.cfg),
		};
		let trace_id = Uuid::new_v4();
		let project_context_description = self
			.resolve_project_context_description(tenant_id.as_str(), project_id.as_str())
			.map(|value| value.to_string());
		let allowed_scopes = ranking::resolve_scopes(&self.cfg, read_profile.as_str())?;
		let policies = self.resolve_finish_search_policies(ranking_override.as_ref())?;

		Ok(RawSearchExecutionContext {
			tenant_id,
			project_id,
			agent_id,
			token_id,
			top_k,
			candidate_k,
			requested_candidate_k,
			effective_candidate_k,
			filter,
			query,
			read_profile,
			record_hits_enabled,
			ranking_override,
			retrieval_sources_policy,
			expansion_mode,
			trace_id,
			project_context_description,
			allowed_scopes,
			policies,
		})
	}

	fn build_raw_planned_response(
		&self,
		context: &RawSearchExecutionContext,
		path: RawSearchPath,
		response: SearchResponse,
		expanded_queries: Vec<String>,
		dynamic_gate: DynamicGateSummary,
	) -> SearchRawPlannedResponse {
		let query_plan = self.build_query_plan(BuildQueryPlanArgs {
			path,
			query: context.query.as_str(),
			tenant_id: context.tenant_id.as_str(),
			project_id: context.project_id.as_str(),
			agent_id: context.agent_id.as_str(),
			read_profile: context.read_profile.as_str(),
			allowed_scopes: &context.allowed_scopes,
			expansion_mode: context.expansion_mode,
			expanded_queries,
			top_k: context.top_k,
			candidate_k: context.candidate_k,
			retrieval_sources_policy: &context.retrieval_sources_policy,
			recursive_enabled: self.cfg.search.recursive.enabled,
			policies: &context.policies,
			dynamic_gate,
		});

		SearchRawPlannedResponse { trace_id: response.trace_id, items: response.items, query_plan }
	}

	async fn maybe_finish_dynamic_search(
		&self,
		args: MaybeDynamicSearchArgs<'_>,
	) -> Result<(Option<Vec<f32>>, Option<SearchResponse>, DynamicGateSummary)> {
		if !args.enabled {
			return Ok((None, None, DynamicGateSummary::default()));
		}

		let query_vec =
			self.embed_single_query(args.query, args.project_context_description).await?;
		let baseline_points = self
			.run_fusion_query(
				&[QueryEmbedding { text: args.query.to_string(), vector: query_vec.clone() }],
				args.filter,
				args.candidate_k,
			)
			.await?;
		let top_score = baseline_points.first().map(|point| point.score).unwrap_or(0.0);
		let fusion_candidates = ranking::collect_chunk_candidates(
			&baseline_points,
			self.cfg.search.prefilter.max_candidates,
			args.candidate_k,
		);
		let should_expand = ranking::should_expand_dynamic(
			baseline_points.len(),
			top_score,
			&self.cfg.search.dynamic,
		);
		let dynamic_gate = DynamicGateSummary {
			considered: true,
			should_expand: Some(should_expand),
			observed_candidates: Some(baseline_points.len() as u32),
			observed_top_score: Some(top_score),
		};

		if should_expand {
			return Ok((Some(query_vec), None, dynamic_gate));
		}

		let StructuredFieldRetrievalResult {
			candidates: structured_candidates,
			structured_matches,
		} = self
			.retrieve_structured_field_candidates(StructuredFieldRetrievalArgs {
				tenant_id: args.tenant_id,
				project_id: args.project_id,
				agent_id: args.agent_id,
				allowed_scopes: args.allowed_scopes,
				query_vec: query_vec.as_slice(),
				candidate_k: args.candidate_k,
				now: OffsetDateTime::now_utc(),
			})
			.await?;
		let mut seed_candidates =
			Vec::with_capacity(fusion_candidates.len() + structured_candidates.len());

		seed_candidates.extend_from_slice(fusion_candidates.as_slice());
		seed_candidates.extend_from_slice(structured_candidates.as_slice());

		let recursive = self
			.run_recursive_retrieval(RecursiveRetrievalArgs {
				query: args.query,
				query_vec: query_vec.as_slice(),
				filter: args.filter,
				candidate_k: args.candidate_k,
				retrieval_sources_policy: args.retrieval_sources_policy,
				seed_candidates: seed_candidates.as_slice(),
			})
			.await?;
		let mut retrieval_sources = vec![
			RetrievalSourceCandidates {
				source: RetrievalSourceKind::Fusion,
				candidates: fusion_candidates,
			},
			RetrievalSourceCandidates {
				source: RetrievalSourceKind::StructuredField,
				candidates: structured_candidates,
			},
		];

		if recursive.enabled {
			retrieval_sources.push(RetrievalSourceCandidates {
				source: RetrievalSourceKind::Recursive,
				candidates: recursive.candidates.clone(),
			});
		}

		let merged_candidates = ranking::merge_retrieval_candidates(
			retrieval_sources,
			args.retrieval_sources_policy,
			args.candidate_k,
		);
		let response = self
			.finish_search(FinishSearchArgs {
				path: args.path,
				trace_id: args.trace_id,
				query: args.query,
				tenant_id: args.tenant_id,
				project_id: args.project_id,
				agent_id: args.agent_id,
				token_id: args.token_id,
				read_profile: args.read_profile,
				allowed_scopes: args.allowed_scopes,
				expanded_queries: vec![args.query.to_string()],
				expansion_mode: ExpansionMode::Dynamic,
				candidates: merged_candidates,
				structured_matches,
				recursive_retrieval: Some(recursive),
				top_k: args.top_k,
				record_hits_enabled: args.record_hits_enabled,
				ranking_override: args.ranking_override.cloned(),
				filter: args.service_filter,
				requested_candidate_k: args.requested_candidate_k,
				effective_candidate_k: args.effective_candidate_k,
			})
			.await?;

		Ok((Some(query_vec), Some(response), dynamic_gate))
	}

	async fn retrieve_search_candidates(
		&self,
		args: SearchRetrievalArgs<'_>,
	) -> Result<SearchRetrievalResult> {
		let queries = match args.expansion_mode {
			ExpansionMode::Off => vec![args.query.to_string()],
			ExpansionMode::Always | ExpansionMode::Dynamic => self.expand_queries(args.query).await,
		};
		let expanded_queries = queries.clone();
		let query_embeddings = self
			.embed_queries(
				queries.as_slice(),
				args.query,
				args.baseline_vector,
				args.project_context_description,
			)
			.await?;
		let fusion_points =
			self.run_fusion_query(&query_embeddings, args.filter, args.candidate_k).await?;
		let fusion_candidates = ranking::collect_chunk_candidates(
			&fusion_points,
			self.cfg.search.prefilter.max_candidates,
			args.candidate_k,
		);
		let original_query_vec = query_embeddings
			.iter()
			.find(|embedded| embedded.text == args.query)
			.map(|embedded| embedded.vector.clone())
			.unwrap_or_else(Vec::new);
		let original_query_vec = if original_query_vec.is_empty() {
			self.embed_single_query(args.query, args.project_context_description).await?
		} else {
			original_query_vec
		};
		let StructuredFieldRetrievalResult {
			candidates: structured_candidates,
			structured_matches,
		} = self
			.retrieve_structured_field_candidates(StructuredFieldRetrievalArgs {
				tenant_id: args.tenant_id,
				project_id: args.project_id,
				agent_id: args.agent_id,
				allowed_scopes: args.allowed_scopes,
				query_vec: original_query_vec.as_slice(),
				candidate_k: args.candidate_k,
				now: OffsetDateTime::now_utc(),
			})
			.await?;
		let mut seed_candidates =
			Vec::with_capacity(fusion_candidates.len() + structured_candidates.len());

		seed_candidates.extend_from_slice(fusion_candidates.as_slice());
		seed_candidates.extend_from_slice(structured_candidates.as_slice());

		let recursive = self
			.run_recursive_retrieval(RecursiveRetrievalArgs {
				query: args.query,
				query_vec: original_query_vec.as_slice(),
				filter: args.filter,
				candidate_k: args.candidate_k,
				retrieval_sources_policy: args.retrieval_sources_policy,
				seed_candidates: seed_candidates.as_slice(),
			})
			.await?;
		let mut retrieval_sources = vec![
			RetrievalSourceCandidates {
				source: RetrievalSourceKind::Fusion,
				candidates: fusion_candidates,
			},
			RetrievalSourceCandidates {
				source: RetrievalSourceKind::StructuredField,
				candidates: structured_candidates,
			},
		];

		if recursive.enabled {
			retrieval_sources.push(RetrievalSourceCandidates {
				source: RetrievalSourceKind::Recursive,
				candidates: recursive.candidates.clone(),
			});
		}

		let merged_candidates = ranking::merge_retrieval_candidates(
			retrieval_sources,
			args.retrieval_sources_policy,
			args.candidate_k,
		);

		Ok(SearchRetrievalResult {
			expanded_queries,
			candidates: merged_candidates,
			structured_matches,
			recursive: Some(recursive),
		})
	}

	async fn run_recursive_retrieval(
		&self,
		args: RecursiveRetrievalArgs<'_>,
	) -> Result<RecursiveRetrievalResult> {
		let recursive_config = &self.cfg.search.recursive;
		let mut result = RecursiveRetrievalResult {
			enabled: recursive_config.enabled
				&& args.retrieval_sources_policy.recursive_weight > 0.0,
			..Default::default()
		};

		if !result.enabled {
			result.stop_reason = Some("disabled".to_string());

			return Ok(result);
		}
		if args.query_vec.is_empty() {
			result.stop_reason = Some("missing_query_vector".to_string());

			return Ok(result);
		}

		let mut seed_scopes = HashSet::<String>::new();

		for candidate in args.seed_candidates {
			if let Some(scope) = candidate.scope.as_deref()
				&& !scope.trim().is_empty()
			{
				seed_scopes.insert(scope.to_string());
			}
		}

		result.scopes_seeded = seed_scopes.len();
		result.candidates_before = args.seed_candidates.len();

		if seed_scopes.is_empty() {
			result.stop_reason = Some("no_scope_seed".to_string());

			return Ok(result);
		}

		let max_depth = recursive_config.max_depth;
		let max_children_per_node =
			usize::try_from(recursive_config.max_children_per_node).unwrap_or(usize::MAX);
		let max_nodes_per_scope =
			usize::try_from(recursive_config.max_nodes_per_scope).unwrap_or(usize::MAX);
		let max_total_nodes =
			usize::try_from(recursive_config.max_total_nodes).unwrap_or(usize::MAX);
		let child_query_embedding =
			QueryEmbedding { text: args.query.to_string(), vector: args.query_vec.to_vec() };
		let per_query_candidate_k =
			args.candidate_k.min(recursive_config.max_nodes_per_scope).max(1);
		let (candidates, queried_scopes, rounds_executed, stop_reason) = self
			.collect_recursive_candidates(
				&args,
				seed_scopes,
				child_query_embedding,
				max_depth,
				max_children_per_node,
				max_nodes_per_scope,
				max_total_nodes,
				per_query_candidate_k,
				self.cfg.search.prefilter.max_candidates,
			)
			.await?;

		result.scopes_queried = queried_scopes;
		result.rounds_executed = rounds_executed;
		result.total_queries = rounds_executed;
		result.candidates = candidates;
		result.candidates_added = result.candidates.len();
		result.candidates_after = result.candidates_before + result.candidates_added;
		result.stop_reason = stop_reason.or(Some("converged".to_string()));

		Ok(result)
	}

	#[allow(clippy::too_many_arguments)]
	async fn collect_recursive_candidates(
		&self,
		args: &RecursiveRetrievalArgs<'_>,
		seed_scopes: HashSet<String>,
		child_query_embedding: QueryEmbedding,
		max_depth: u32,
		max_children_per_node: usize,
		max_nodes_per_scope: usize,
		max_total_nodes: usize,
		per_query_candidate_k: u32,
		prefilter_max_candidates: u32,
	) -> Result<(Vec<ChunkCandidate>, usize, u32, Option<String>)> {
		let mut queued_scopes: VecDeque<(String, u32)> = VecDeque::new();
		let mut discovered_scopes = seed_scopes.clone();
		let mut recursion_candidates = Vec::<ChunkCandidate>::new();
		let mut seen_chunks =
			args.seed_candidates.iter().map(|candidate| candidate.chunk_id).collect::<HashSet<_>>();
		let mut scope_counts: HashMap<String, u32> = HashMap::new();
		let mut queried_scopes = 0_usize;
		let mut rounds_executed = 0_u32;
		let mut stop_reason: Option<String> = None;

		for scope in seed_scopes {
			queued_scopes.push_back((scope, 1));
		}

		while let Some((scope, depth)) = queued_scopes.pop_front() {
			if depth > max_depth {
				stop_reason = Some("max_depth".to_string());

				break;
			}

			queried_scopes = queried_scopes.saturating_add(1);
			rounds_executed = rounds_executed.saturating_add(1);

			let mut scoped_filter = args.filter.clone();

			scoped_filter.must.push(Condition::matches("scope", scope.clone()));

			let recursive_points = self
				.run_fusion_query(
					std::slice::from_ref(&child_query_embedding),
					&scoped_filter,
					per_query_candidate_k,
				)
				.await?;
			let scope_query_limit = per_query_candidate_k.min(max_nodes_per_scope as u32);
			let recursive_candidates_for_scope = ranking::collect_chunk_candidates(
				&recursive_points,
				prefilter_max_candidates.min(scope_query_limit),
				scope_query_limit,
			);
			let mut child_scopes = HashSet::<String>::new();

			for mut candidate in recursive_candidates_for_scope {
				if recursion_candidates.len() >= max_total_nodes {
					stop_reason = Some("max_total_nodes".to_string());

					break;
				}

				let scope_key = candidate.scope.clone().unwrap_or_else(|| scope.clone());
				let scope_count = scope_counts.entry(scope_key.clone()).or_default();

				if (*scope_count as usize) >= max_nodes_per_scope {
					continue;
				}
				if !seen_chunks.insert(candidate.chunk_id) {
					continue;
				}

				*scope_count = scope_count.saturating_add(1);
				candidate.scope = Some(scope_key.clone());

				recursion_candidates.push(candidate);

				if depth < max_depth
					&& child_scopes.len() < max_children_per_node
					&& !scope_key.is_empty()
					&& discovered_scopes.insert(scope_key.clone())
				{
					child_scopes.insert(scope_key.clone());
					queued_scopes.push_back((scope_key.clone(), depth.saturating_add(1)));
				}
			}

			if stop_reason.is_some() {
				break;
			}
		}

		Ok((recursion_candidates, queried_scopes, rounds_executed, stop_reason))
	}

	fn resolve_project_context_description<'a>(
		&'a self,
		tenant_id: &str,
		project_id: &str,
	) -> Option<&'a str> {
		let context = self.cfg.context.as_ref()?;
		let descriptions = context.project_descriptions.as_ref()?;
		let key = format!("{tenant_id}:{project_id}");
		let mut saw_non_english = false;

		if let Some(value) = descriptions.get(&key) {
			let trimmed = value.trim();

			if !trimmed.is_empty() {
				if !elf_domain::english_gate::is_english_natural_language(trimmed) {
					saw_non_english = true;
				} else {
					return Some(trimmed);
				}
			}
		}
		if let Some(value) = descriptions.get(project_id) {
			let trimmed = value.trim();

			if !trimmed.is_empty() {
				if !elf_domain::english_gate::is_english_natural_language(trimmed) {
					saw_non_english = true;
				} else {
					return Some(trimmed);
				}
			}
		}

		if saw_non_english {
			tracing::warn!(
				tenant_id = %tenant_id,
				project_id = %project_id,
				"Project context description is non-English. Skipping context."
			);
		}

		None
	}

	pub async fn search_explain(&self, req: SearchExplainRequest) -> Result<SearchExplainResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id and project_id are required.".to_string(),
			});
		}

		let row = sqlx::query_as::<_, SearchExplainTraceRow>(
			"\
SELECT
	t.trace_id,
	t.tenant_id,
	t.project_id,
	t.agent_id,
	t.read_profile,
	t.query,
	t.expansion_mode,
	t.expanded_queries,
	t.allowed_scopes,
	t.candidate_count,
	t.top_k,
	t.config_snapshot,
	t.trace_version,
	t.created_at,
	i.item_id,
	i.note_id,
	i.chunk_id,
	i.rank,
	i.explain
FROM search_trace_items i
JOIN search_traces t ON i.trace_id = t.trace_id

WHERE i.item_id = $1 AND t.tenant_id = $2 AND t.project_id = $3",
		)
		.bind(req.result_handle)
		.bind(tenant_id)
		.bind(project_id)
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
		let trajectory = load_item_trajectory(&self.db.pool, row.trace_id, row.item_id).await?;

		Ok(SearchExplainResponse { trace, item, trajectory })
	}

	pub async fn trace_get(&self, req: TraceGetRequest) -> Result<TraceGetResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();

		if req.agent_id.trim().is_empty() {
			return Err(Error::InvalidRequest { message: "agent_id is required.".to_string() });
		}
		if tenant_id.is_empty() || project_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id and project_id are required.".to_string(),
			});
		}

		let row = sqlx::query_as::<_, SearchTraceRow>(
			"\
SELECT
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
	created_at
FROM search_traces
WHERE trace_id = $1 AND tenant_id = $2 AND project_id = $3",
		)
		.bind(req.trace_id)
		.bind(tenant_id)
		.bind(project_id)
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
		let item_rows = sqlx::query_as::<_, SearchTraceItemRow>(
			"\
SELECT
	item_id,
	note_id,
	chunk_id,
	rank,
	explain
FROM search_trace_items
WHERE trace_id = $1
ORDER BY rank ASC",
		)
		.bind(req.trace_id)
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

		let trajectory_summary = load_trace_trajectory_summary(&self.db.pool, req.trace_id).await?;

		Ok(TraceGetResponse { trace, items, trajectory_summary })
	}

	pub async fn trace_trajectory_get(
		&self,
		req: TraceTrajectoryGetRequest,
	) -> Result<SearchTrajectoryResponse> {
		let base = self
			.trace_get(TraceGetRequest {
				tenant_id: req.tenant_id,
				project_id: req.project_id,
				agent_id: req.agent_id,
				trace_id: req.trace_id,
			})
			.await?;
		let stages = load_trace_trajectory_stages(&self.db.pool, req.trace_id).await?;
		let trajectory = build_trajectory_summary_from_stages(stages.as_slice());

		Ok(SearchTrajectoryResponse { trace: base.trace, trajectory, stages })
	}

	pub async fn trace_recent_list(
		&self,
		req: TraceRecentListRequest,
	) -> Result<TraceRecentListResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let caller_agent_id = req.agent_id.trim();
		let cursor_created_at = req.cursor_created_at;
		let cursor_trace_id = req.cursor_trace_id;
		let agent_id_filter = req.agent_id_filter.map(|value| value.trim().to_string());
		let read_profile = req.read_profile.map(|value| value.trim().to_string());
		let limit = req.limit.unwrap_or(DEFAULT_RECENT_TRACES_LIMIT);

		if cursor_created_at.is_some() != cursor_trace_id.is_some() {
			return Err(Error::InvalidRequest {
				message: "cursor_created_at and cursor_trace_id must be both set or both omitted."
					.to_string(),
			});
		}
		if caller_agent_id.is_empty() {
			return Err(Error::InvalidRequest { message: "agent_id is required.".to_string() });
		}
		if tenant_id.is_empty() || project_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id and project_id are required.".to_string(),
			});
		}
		if limit == 0 || limit > MAX_RECENT_TRACES_LIMIT {
			return Err(Error::InvalidRequest {
				message: format!("limit must be between 1 and {MAX_RECENT_TRACES_LIMIT}."),
			});
		}

		if let (Some(created_after), Some(created_before)) = (req.created_after, req.created_before)
			&& created_after >= created_before
		{
			return Err(Error::InvalidRequest {
				message: "created_after must be before created_before.".to_string(),
			});
		}

		let agent_id_filter = agent_id_filter.as_deref();
		let read_profile = read_profile.as_deref();
		let fetch_limit = (limit + 1).min(MAX_RECENT_TRACES_LIMIT + 1);
		let rows = sqlx::query_as::<_, SearchRecentTraceRow>(
			"\
SELECT
\ttrace_id,
\ttenant_id,
\tproject_id,
\tagent_id,
\tread_profile,
\tquery,
\tcreated_at
FROM search_traces
WHERE tenant_id = $1
\tAND project_id = $2
\tAND ($3::text IS NULL OR agent_id = $3)
\tAND ($4::text IS NULL OR read_profile = $4)
\tAND ($5::timestamptz IS NULL OR created_at > $5)
\tAND ($6::timestamptz IS NULL OR created_at < $6)
\tAND ($7::timestamptz IS NULL OR $8::uuid IS NULL OR (created_at, trace_id) < ($7, $8))
ORDER BY created_at DESC, trace_id DESC
LIMIT $9
",
		)
		.bind(tenant_id)
		.bind(project_id)
		.bind(agent_id_filter)
		.bind(read_profile)
		.bind(req.created_after)
		.bind(req.created_before)
		.bind(cursor_created_at)
		.bind(cursor_trace_id)
		.bind(fetch_limit as i64)
		.fetch_all(&self.db.pool)
		.await?;
		let next_cursor = if rows.len() > limit as usize {
			let cursor_row = &rows[limit as usize];

			Some(TraceRecentCursor {
				created_at: cursor_row.created_at,
				trace_id: cursor_row.trace_id,
			})
		} else {
			None
		};
		let mut response_rows = rows;

		response_rows.truncate(limit as usize);

		let mut traces = Vec::with_capacity(response_rows.len());

		for row in response_rows {
			traces.push(RecentTraceHeader {
				trace_id: row.trace_id,
				tenant_id: row.tenant_id,
				project_id: row.project_id,
				agent_id: row.agent_id,
				read_profile: row.read_profile,
				query: row.query,
				created_at: row.created_at,
			});
		}

		Ok(TraceRecentListResponse {
			schema: RECENT_TRACES_SCHEMA_V1.to_string(),
			traces,
			next_cursor,
		})
	}

	pub async fn trace_bundle_get(
		&self,
		req: TraceBundleGetRequest,
	) -> Result<TraceBundleResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();

		if req.agent_id.trim().is_empty() {
			return Err(Error::InvalidRequest { message: "agent_id is required.".to_string() });
		}
		if tenant_id.is_empty() || project_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id and project_id are required.".to_string(),
			});
		}

		let base = self
			.trace_get(TraceGetRequest {
				tenant_id: tenant_id.to_string(),
				project_id: project_id.to_string(),
				agent_id: req.agent_id.trim().to_string(),
				trace_id: req.trace_id,
			})
			.await?;
		let default_stage_items_limit = match req.mode {
			TraceBundleMode::Bounded => DEFAULT_BOUNDED_STAGE_ITEMS_LIMIT,
			TraceBundleMode::Full => DEFAULT_FULL_STAGE_ITEMS_LIMIT,
		};
		let default_candidates_limit = match req.mode {
			TraceBundleMode::Bounded => DEFAULT_BOUNDED_CANDIDATES_LIMIT,
			TraceBundleMode::Full => DEFAULT_FULL_CANDIDATES_LIMIT,
		};
		let stage_items_limit = req
			.stage_items_limit
			.unwrap_or(default_stage_items_limit)
			.min(MAX_TRACE_BUNDLE_ITEMS_LIMIT);
		let candidates_limit = req
			.candidates_limit
			.unwrap_or(default_candidates_limit)
			.min(MAX_TRACE_BUNDLE_CANDIDATES_LIMIT);
		let mut stages = load_trace_trajectory_stages(&self.db.pool, req.trace_id).await?;

		for stage in stages.iter_mut() {
			stage.items.truncate(stage_items_limit as usize);
		}

		let candidates = if candidates_limit == 0 {
			None
		} else {
			let candidate_rows = sqlx::query_as::<_, TraceCandidateSnapshotRow>(
				"\
SELECT candidate_snapshot
FROM search_trace_candidates
WHERE trace_id = $1
ORDER BY retrieval_rank ASC, candidate_id ASC
LIMIT $2
",
			)
			.bind(req.trace_id)
			.bind(candidates_limit as i32)
			.fetch_all(&self.db.pool)
			.await?;
			let mut candidates = Vec::with_capacity(candidate_rows.len());

			for row in candidate_rows {
				candidates
					.push(ranking::decode_json(row.candidate_snapshot, "candidate_snapshot")?);
			}

			if candidates.is_empty() { None } else { Some(candidates) }
		};

		Ok(TraceBundleResponse {
			schema: TRACE_BUNDLE_SCHEMA_V1.to_string(),
			generated_at: OffsetDateTime::now_utc(),
			trace: base.trace,
			items: base.items,
			trajectory_summary: base.trajectory_summary,
			stages,
			candidates,
		})
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

		if let Some(key) = cache_key.as_ref()
			&& let Some(queries) = self.read_expansion_cache_queries(key, cache_cfg, now).await
		{
			return queries;
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
			self.store_expansion_cache_queries(&key, &result, cache_cfg).await;
		}

		result
	}

	async fn read_expansion_cache_queries(
		&self,
		key: &str,
		cache_cfg: &SearchCache,
		now: OffsetDateTime,
	) -> Option<Vec<String>> {
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

				let cached: ExpansionCachePayload = match serde_json::from_value(payload.value) {
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

				(!cached.queries.is_empty()).then_some(cached.queries)
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

				None
			},
			Err(err) => {
				tracing::warn!(
					error = %err,
					cache_kind = CacheKind::Expansion.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					"Cache read failed."
				);

				None
			},
		}
	}

	async fn store_expansion_cache_queries(
		&self,
		key: &str,
		queries: &[String],
		cache_cfg: &SearchCache,
	) {
		let payload = ExpansionCachePayload { queries: queries.to_vec() };
		let payload_json = match serde_json::to_value(&payload) {
			Ok(value) => value,
			Err(err) => {
				tracing::warn!(
					error = %err,
					cache_kind = CacheKind::Expansion.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					"Cache payload encode failed."
				);

				return;
			},
		};
		let stored_at = OffsetDateTime::now_utc();
		let expires_at = stored_at + Duration::days(cache_cfg.expansion_ttl_days);

		match store_cache_payload(
			&self.db.pool,
			CacheKind::Expansion,
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
					cache_kind = CacheKind::Expansion.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					hit = false,
					payload_size,
					ttl_days = cache_cfg.expansion_ttl_days,
					"Cache stored."
				);
			},
			Ok(None) => {
				tracing::warn!(
					cache_kind = CacheKind::Expansion.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
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
					cache_key_prefix = ranking::cache_key_prefix(key),
					"Cache write failed."
				);
			},
		}
	}

	async fn retrieve_structured_field_candidates(
		&self,
		args: StructuredFieldRetrievalArgs<'_>,
	) -> Result<StructuredFieldRetrievalResult> {
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
		let rows = self
			.fetch_structured_field_hits(StructuredFieldHitArgs {
				embed_version: embed_version.as_str(),
				tenant_id,
				project_id,
				agent_id,
				now,
				vec_text: vec_text.as_str(),
				retrieval_limit,
				private_allowed,
				non_private_scopes: non_private_scopes.as_slice(),
			})
			.await?;
		let (ordered_note_ids, structured_matches_out) = build_structured_field_matches(rows);

		if ordered_note_ids.is_empty() {
			return Ok(StructuredFieldRetrievalResult {
				candidates: Vec::new(),
				structured_matches: structured_matches_out,
			});
		}

		let best_by_note = self
			.fetch_best_chunks_for_notes(
				embed_version.as_str(),
				ordered_note_ids.as_slice(),
				vec_text.as_str(),
			)
			.await?;
		let structured_candidates = build_structured_field_candidates(
			candidate_k,
			ordered_note_ids,
			best_by_note,
			embed_version.as_str(),
		);

		Ok(StructuredFieldRetrievalResult {
			candidates: structured_candidates,
			structured_matches: structured_matches_out,
		})
	}

	async fn fetch_structured_field_hits(
		&self,
		args: StructuredFieldHitArgs<'_>,
	) -> Result<Vec<FieldHit>> {
		if args.private_allowed && args.non_private_scopes.is_empty() {
			self.fetch_structured_field_hits_private_only(args).await
		} else if !args.private_allowed {
			self.fetch_structured_field_hits_non_private_only(args).await
		} else {
			self.fetch_structured_field_hits_mixed(args).await
		}
	}

	async fn fetch_structured_field_hits_private_only(
		&self,
		args: StructuredFieldHitArgs<'_>,
	) -> Result<Vec<FieldHit>> {
		let rows = sqlx::query_as::<_, StructuredFieldHitRow>(
			"\
SELECT
	f.note_id,
	f.field_kind
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
		)
		.bind(args.embed_version)
		.bind(args.tenant_id)
		.bind(args.project_id)
		.bind(args.now)
		.bind(args.agent_id)
		.bind(args.vec_text)
		.bind(args.retrieval_limit)
		.fetch_all(&self.db.pool)
		.await?;

		Ok(rows
			.into_iter()
			.map(|row| FieldHit { note_id: row.note_id, field_kind: row.field_kind })
			.collect())
	}

	async fn fetch_structured_field_hits_non_private_only(
		&self,
		args: StructuredFieldHitArgs<'_>,
	) -> Result<Vec<FieldHit>> {
		let rows = sqlx::query_as::<_, StructuredFieldHitRow>(
			"\
SELECT
	f.note_id,
	f.field_kind
FROM memory_note_fields f
JOIN note_field_embeddings e
	ON e.field_id = f.field_id
	AND e.embedding_version = $1
JOIN memory_notes n
	ON n.note_id = f.note_id
WHERE n.tenant_id = $2
	AND (n.project_id = $3 OR (n.project_id = $8 AND n.scope = 'org_shared'))
	AND n.status = 'active'
	AND (n.expires_at IS NULL OR n.expires_at > $4)
	AND n.scope = ANY($5::text[])
ORDER BY e.vec <=> $6::text::vector ASC
LIMIT $7",
		)
		.bind(args.embed_version)
		.bind(args.tenant_id)
		.bind(args.project_id)
		.bind(args.now)
		.bind(args.non_private_scopes)
		.bind(args.vec_text)
		.bind(args.retrieval_limit)
		.bind(access::ORG_PROJECT_ID)
		.fetch_all(&self.db.pool)
		.await?;

		Ok(rows
			.into_iter()
			.map(|row| FieldHit { note_id: row.note_id, field_kind: row.field_kind })
			.collect())
	}

	async fn fetch_structured_field_hits_mixed(
		&self,
		args: StructuredFieldHitArgs<'_>,
	) -> Result<Vec<FieldHit>> {
		let rows = sqlx::query_as::<_, StructuredFieldHitRow>(
			"\
SELECT
	f.note_id,
	f.field_kind
FROM memory_note_fields f
JOIN note_field_embeddings e
	ON e.field_id = f.field_id
	AND e.embedding_version = $1
JOIN memory_notes n
	ON n.note_id = f.note_id
WHERE n.tenant_id = $2
	AND (n.project_id = $3 OR (n.project_id = $9 AND n.scope = 'org_shared'))
	AND n.status = 'active'
	AND (n.expires_at IS NULL OR n.expires_at > $4)
	AND (
		(n.scope = 'agent_private' AND n.agent_id = $5)
		OR n.scope = ANY($6::text[])
	)
ORDER BY e.vec <=> $7::text::vector ASC
LIMIT $8",
		)
		.bind(args.embed_version)
		.bind(args.tenant_id)
		.bind(args.project_id)
		.bind(args.now)
		.bind(args.agent_id)
		.bind(args.non_private_scopes)
		.bind(args.vec_text)
		.bind(args.retrieval_limit)
		.bind(access::ORG_PROJECT_ID)
		.fetch_all(&self.db.pool)
		.await?;

		Ok(rows
			.into_iter()
			.map(|row| FieldHit { note_id: row.note_id, field_kind: row.field_kind })
			.collect())
	}

	async fn fetch_best_chunks_for_notes(
		&self,
		embed_version: &str,
		ordered_note_ids: &[Uuid],
		vec_text: &str,
	) -> Result<HashMap<Uuid, (Uuid, i32)>> {
		let best_chunks = sqlx::query_as::<_, BestChunkForNoteRow>(
			"\
SELECT DISTINCT ON (c.note_id)
	c.note_id,
	c.chunk_id,
	c.chunk_index
FROM memory_note_chunks c
JOIN note_chunk_embeddings e
	ON e.chunk_id = c.chunk_id
	AND e.embedding_version = $1
WHERE c.note_id = ANY($2::uuid[])
ORDER BY c.note_id ASC, e.vec <=> $3::text::vector ASC",
		)
		.bind(embed_version)
		.bind(ordered_note_ids)
		.bind(vec_text)
		.fetch_all(&self.db.pool)
		.await?;
		let mut best_by_note = HashMap::new();

		for row in best_chunks {
			best_by_note.insert(row.note_id, (row.chunk_id, row.chunk_index));
		}

		Ok(best_by_note)
	}

	async fn finish_search(&self, args: FinishSearchArgs<'_>) -> Result<SearchResponse> {
		let now = OffsetDateTime::now_utc();
		let candidate_count = args.candidates.len();
		let candidate_note_ids: Vec<Uuid> =
			args.candidates.iter().map(|candidate| candidate.note_id).collect();
		let policies = self.resolve_finish_search_policies(args.ranking_override.as_ref())?;
		let note_meta = self
			.fetch_note_meta_for_candidates(
				args.tenant_id,
				args.project_id,
				args.agent_id,
				args.allowed_scopes,
				candidate_note_ids.as_slice(),
				now,
			)
			.await?;
		let scoring = self
			.build_finish_search_scoring(
				args.query,
				args.candidates,
				&note_meta,
				&policies,
				args.top_k,
				candidate_count,
				args.filter,
				args.requested_candidate_k,
				args.effective_candidate_k,
				now,
			)
			.await?;
		let FinishSearchScoringResult {
			query_tokens,
			filtered_candidates,
			scored_count,
			snippet_count,
			filtered_candidate_count,
			filter_impact,
			mut trace_candidates,
			fused_results,
			selected_results,
			diversity_decisions,
			selected_count,
		} = scoring;
		let relation_contexts = self
			.build_relation_context_for_selected_results(
				&selected_results,
				args.tenant_id,
				args.project_id,
				args.agent_id,
				args.allowed_scopes,
				now,
			)
			.await?;

		ranking::attach_diversity_decisions_to_trace_candidates(
			&mut trace_candidates,
			&diversity_decisions,
		);

		self.record_hits_if_enabled(args.record_hits_enabled, args.query, &selected_results, now)
			.await?;

		let items = self
			.build_items_and_write_trace(BuildTraceArgs {
				path: args.path,
				trace_id: args.trace_id,
				query: args.query,
				tenant_id: args.tenant_id,
				project_id: args.project_id,
				agent_id: args.agent_id,
				token_id: args.token_id,
				read_profile: args.read_profile,
				expansion_mode: args.expansion_mode,
				expanded_queries: args.expanded_queries,
				allowed_scopes: args.allowed_scopes,
				candidate_count,
				filtered_candidate_count,
				snippet_count,
				scored_count,
				fused_count: fused_results.len(),
				selected_count,
				top_k: args.top_k,
				query_tokens: query_tokens.as_slice(),
				structured_matches: &args.structured_matches,
				policies: &policies,
				diversity_decisions: &diversity_decisions,
				recall_candidates: filtered_candidates,
				fused_results,
				selected_results,
				relation_contexts,
				trace_candidates,
				recursive_retrieval: args.recursive_retrieval.as_ref(),
				now,
				ranking_override: &args.ranking_override,
				filter_impact,
			})
			.await?;

		Ok(SearchResponse { trace_id: args.trace_id, items })
	}

	async fn build_items_and_write_trace(
		&self,
		args: BuildTraceArgs<'_>,
	) -> Result<Vec<SearchItem>> {
		let trace_id = args.trace_id;
		let (items, trace_payload) = self.build_items_and_trace_payload(args);

		self.write_trace_payload(trace_id, trace_payload).await?;

		Ok(items)
	}

	#[allow(clippy::too_many_arguments)]
	async fn build_finish_search_scoring(
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
		let det_query_tokens = build_deterministic_query_tokens(&self.cfg, query);
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
			})
			.await?;
		let scored_count = scored.len();
		let trace_candidates = self.build_trace_candidates(&scored, now);
		let results = select_best_scored_chunks(scored);
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

	fn apply_filter_to_candidates(
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

	async fn build_relation_context_for_selected_results(
		&self,
		selected_results: &[ScoredChunk],
		tenant_id: &str,
		project_id: &str,
		agent_id: &str,
		allowed_scopes: &[String],
		now: OffsetDateTime,
	) -> Result<HashMap<Uuid, Vec<SearchExplainRelationContext>>> {
		if !self.cfg.search.graph_context.enabled {
			return Ok(HashMap::new());
		}

		let selected_note_ids: Vec<Uuid> =
			selected_results.iter().map(|chunk| chunk.item.note.note_id).collect();

		if selected_note_ids.is_empty() {
			return Ok(HashMap::new());
		}

		self.fetch_relation_contexts_for_notes(
			selected_note_ids.as_slice(),
			tenant_id,
			project_id,
			agent_id,
			allowed_scopes,
			now,
		)
		.await
	}

	fn resolve_finish_search_policies(
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

	fn build_query_plan(&self, args: BuildQueryPlanArgs<'_>) -> QueryPlan {
		let allowed_scopes = sorted_unique_strings(args.allowed_scopes.to_vec());
		let expanded_queries = sorted_unique_strings(args.expanded_queries);
		let retrieval_stages = self.build_query_plan_retrieval_stages(
			args.candidate_k,
			args.retrieval_sources_policy,
			args.recursive_enabled,
		);
		let rewrite =
			self.build_query_plan_rewrite(args.expansion_mode, expanded_queries, args.dynamic_gate);
		let fusion_policy = self.build_query_plan_fusion_policy(args.retrieval_sources_policy);
		let rerank_policy = self.build_query_plan_rerank_policy(args.policies);
		let budget = self.build_query_plan_budget(args.top_k, args.candidate_k);
		let stages = Self::build_query_plan_stages(QueryPlanStagesArgs {
			path: args.path,
			query: args.query,
			read_profile: args.read_profile,
			allowed_scope_count: allowed_scopes.len(),
			rewrite: &rewrite,
			retrieval_stages: &retrieval_stages,
			fusion_policy: &fusion_policy,
			rerank_policy: &rerank_policy,
			budget: &budget,
		});

		QueryPlan {
			schema: QUERY_PLAN_SCHEMA.to_string(),
			version: QUERY_PLAN_VERSION.to_string(),
			stages,
			intent: QueryPlanIntent {
				query: args.query.to_string(),
				tenant_id: args.tenant_id.to_string(),
				project_id: args.project_id.to_string(),
				agent_id: args.agent_id.to_string(),
				read_profile: args.read_profile.to_string(),
				allowed_scopes,
			},
			rewrite,
			retrieval_stages,
			fusion_policy,
			rerank_policy,
			budget,
		}
	}

	fn build_query_plan_retrieval_stages(
		&self,
		candidate_k: u32,
		retrieval_sources_policy: &ResolvedRetrievalSourcesPolicy,
		recursive_enabled: bool,
	) -> Vec<QueryPlanRetrievalStage> {
		let mut stages = vec![
			QueryPlanRetrievalStage {
				name: "fusion_dense_bm25".to_string(),
				source: "qdrant_fusion".to_string(),
				enabled: true,
				candidate_limit: candidate_k,
			},
			QueryPlanRetrievalStage {
				name: "structured_field_vector".to_string(),
				source: "postgres_vector".to_string(),
				enabled: retrieval_sources_policy.structured_field_weight > 0.0,
				candidate_limit: candidate_k,
			},
		];

		if recursive_enabled {
			stages.push(QueryPlanRetrievalStage {
				name: "recursive_scope".to_string(),
				source: "scope_graph".to_string(),
				enabled: retrieval_sources_policy.recursive_weight > 0.0,
				candidate_limit: candidate_k,
			});
		}

		stages
	}

	fn build_query_plan_rewrite(
		&self,
		expansion_mode: ExpansionMode,
		expanded_queries: Vec<String>,
		dynamic_gate: DynamicGateSummary,
	) -> QueryPlanRewrite {
		QueryPlanRewrite {
			expansion_mode: ranking::expansion_mode_label(expansion_mode).to_string(),
			expanded_queries,
			dynamic_gate: QueryPlanDynamicGate {
				considered: dynamic_gate.considered,
				should_expand: dynamic_gate.should_expand,
				observed_candidates: dynamic_gate.observed_candidates,
				observed_top_score: dynamic_gate.observed_top_score,
				min_candidates: self.cfg.search.dynamic.min_candidates,
				min_top_score: self.cfg.search.dynamic.min_top_score,
			},
		}
	}

	fn build_query_plan_fusion_policy(
		&self,
		retrieval_sources_policy: &ResolvedRetrievalSourcesPolicy,
	) -> QueryPlanFusionPolicy {
		QueryPlanFusionPolicy {
			strategy: "weighted_merge".to_string(),
			fusion_weight: retrieval_sources_policy.fusion_weight,
			structured_field_weight: retrieval_sources_policy.structured_field_weight,
			recursive_weight: retrieval_sources_policy.recursive_weight,
			fusion_priority: retrieval_sources_policy.fusion_priority,
			structured_field_priority: retrieval_sources_policy.structured_field_priority,
			recursive_priority: retrieval_sources_policy.recursive_priority,
		}
	}

	fn build_query_plan_rerank_policy(
		&self,
		policies: &FinishSearchPolicies,
	) -> QueryPlanRerankPolicy {
		QueryPlanRerankPolicy {
			provider_id: self.cfg.providers.rerank.provider_id.clone(),
			model: self.cfg.providers.rerank.model.clone(),
			blend_enabled: policies.blend_policy.enabled,
			rerank_normalization: policies.blend_policy.rerank_normalization.as_str().to_string(),
			retrieval_normalization: policies
				.blend_policy
				.retrieval_normalization
				.as_str()
				.to_string(),
			blend_segments: policies
				.blend_policy
				.segments
				.iter()
				.map(|segment| QueryPlanBlendSegment {
					max_retrieval_rank: segment.max_retrieval_rank,
					retrieval_weight: segment.retrieval_weight,
				})
				.collect(),
			diversity_enabled: policies.diversity_policy.enabled,
			diversity_sim_threshold: policies.diversity_policy.sim_threshold,
			diversity_mmr_lambda: policies.diversity_policy.mmr_lambda,
			diversity_max_skips: policies.diversity_policy.max_skips,
		}
	}

	fn build_query_plan_budget(&self, top_k: u32, candidate_k: u32) -> QueryPlanBudget {
		QueryPlanBudget {
			top_k,
			candidate_k,
			prefilter_max_candidates: self.cfg.search.prefilter.max_candidates,
			expansion_max_queries: self.cfg.search.expansion.max_queries,
			cache_enabled: self.cfg.search.cache.enabled,
		}
	}

	fn build_query_plan_stages(args: QueryPlanStagesArgs<'_>) -> Vec<QueryPlanStage> {
		vec![
			QueryPlanStage {
				name: "intent".to_string(),
				details: serde_json::json!({
					"path": raw_search_path_label(args.path),
					"query": args.query,
					"read_profile": args.read_profile,
					"allowed_scope_count": args.allowed_scope_count,
				}),
			},
			QueryPlanStage {
				name: "rewrite".to_string(),
				details: serde_json::json!({
					"expansion_mode": args.rewrite.expansion_mode.as_str(),
					"expanded_query_count": args.rewrite.expanded_queries.len(),
					"dynamic_gate_considered": args.rewrite.dynamic_gate.considered,
					"dynamic_gate_should_expand": args.rewrite.dynamic_gate.should_expand,
				}),
			},
			QueryPlanStage {
				name: "retrieval".to_string(),
				details: serde_json::json!({
					"stages": args.retrieval_stages,
				}),
			},
			QueryPlanStage {
				name: "fusion".to_string(),
				details: serde_json::json!({
					"strategy": args.fusion_policy.strategy.as_str(),
					"fusion_weight": args.fusion_policy.fusion_weight,
					"structured_field_weight": args.fusion_policy.structured_field_weight,
				}),
			},
			QueryPlanStage {
				name: "rerank".to_string(),
				details: serde_json::json!({
					"provider_id": args.rerank_policy.provider_id.as_str(),
					"model": args.rerank_policy.model.as_str(),
					"blend_enabled": args.rerank_policy.blend_enabled,
					"diversity_enabled": args.rerank_policy.diversity_enabled,
				}),
			},
			QueryPlanStage {
				name: "budget".to_string(),
				details: serde_json::json!({
					"top_k": args.budget.top_k,
					"candidate_k": args.budget.candidate_k,
					"prefilter_max_candidates": args.budget.prefilter_max_candidates,
					"expansion_max_queries": args.budget.expansion_max_queries,
					"cache_enabled": args.budget.cache_enabled,
				}),
			},
		]
	}

	async fn score_snippet_items(
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
		} = args;

		if snippet_items.is_empty() {
			return Ok(Vec::new());
		}

		let scores =
			self.rerank_snippet_items(query, snippet_items.as_slice(), cache_cfg, now).await?;
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
			snippet_items.into_iter().zip(scores.into_iter()).zip(rerank_ranks.into_iter())
		{
			scored.push(score_chunk_candidate(&score_ctx, item, rerank_score, rerank_rank));
		}

		Ok(scored)
	}

	fn build_trace_candidates(
		&self,
		scored: &[ScoredChunk],
		now: OffsetDateTime,
	) -> Vec<TraceCandidateRecord> {
		if !self.cfg.search.explain.capture_candidates || scored.is_empty() {
			return Vec::new();
		}

		let candidate_expires_at =
			now + Duration::days(self.cfg.search.explain.candidate_retention_days);

		scored
			.iter()
			.map(|scored_chunk| {
				build_trace_candidate_record(scored_chunk, now, candidate_expires_at)
			})
			.collect()
	}

	async fn apply_diversity_policy(
		&self,
		results: Vec<ScoredChunk>,
		top_k: u32,
		diversity_policy: &ResolvedDiversityPolicy,
	) -> Result<(Vec<ScoredChunk>, HashMap<Uuid, DiversityDecision>)> {
		let note_vectors = if diversity_policy.enabled {
			fetch_note_vectors_for_diversity(&self.db.pool, results.as_slice()).await?
		} else {
			HashMap::new()
		};
		let (selected_results, diversity_decisions) =
			ranking::select_diverse_results(results, top_k, diversity_policy, &note_vectors);

		Ok((selected_results, diversity_decisions))
	}

	async fn record_hits_if_enabled(
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

		record_hits(&mut *tx, query, selected_results, now).await?;

		tx.commit().await?;

		Ok(())
	}

	fn build_items_and_trace_payload(
		&self,
		args: BuildTraceArgs<'_>,
	) -> (Vec<SearchItem>, TracePayload) {
		let mut trajectory_stages = build_trace_trajectory_stages(&args);
		let trace_context = TraceContext {
			trace_id: args.trace_id,
			tenant_id: args.tenant_id,
			project_id: args.project_id,
			agent_id: args.agent_id,
			read_profile: args.read_profile,
			query: args.query,
			expansion_mode: args.expansion_mode,
			expanded_queries: args.expanded_queries.clone(),
			allowed_scopes: args.allowed_scopes,
			candidate_count: args.candidate_count,
			top_k: args.top_k,
		};
		let mut config_snapshot = ranking::build_config_snapshot(
			&self.cfg,
			&args.policies.blend_policy,
			&args.policies.diversity_policy,
			&args.policies.retrieval_sources_policy,
			args.ranking_override.as_ref(),
			args.policies.policy_id.as_str(),
			&args.policies.policy_snapshot,
		);

		if let Some(object) = config_snapshot.as_object_mut() {
			object.insert("audit".to_string(), build_trace_audit(args.agent_id, args.token_id));
		}

		let mut items = Vec::with_capacity(args.selected_results.len());
		let mut trace_builder = SearchTraceBuilder::new(
			trace_context,
			config_snapshot,
			self.cfg.search.explain.retention_days,
			args.now,
		);
		let mut final_stage_items = Vec::new();

		for candidate in args.trace_candidates {
			trace_builder.push_candidate(candidate);
		}
		for (idx, scored_chunk) in args.selected_results.into_iter().enumerate() {
			let rank = idx as u32 + 1;
			let (item, trace_item) = build_search_item_and_trace_item(BuildSearchItemArgs {
				cfg: &self.cfg,
				policy_id: args.policies.policy_id.as_str(),
				blend_policy: &args.policies.blend_policy,
				diversity_policy: &args.policies.diversity_policy,
				diversity_decisions: args.diversity_decisions,
				query_tokens: args.query_tokens,
				structured_matches: args.structured_matches,
				relation_contexts: &args.relation_contexts,
				scored_chunk,
				rank,
			});

			final_stage_items.push(TraceTrajectoryStageItemRecord {
				id: Uuid::new_v4(),
				item_id: Some(item.result_handle),
				note_id: Some(item.note_id),
				chunk_id: Some(item.chunk_id),
				metrics: serde_json::json!({
					"rank": rank,
					"final_score": item.final_score,
				}),
			});
			items.push(item);
			trace_builder.push_item(trace_item);
		}

		if let Some(stage) =
			trajectory_stages.iter_mut().find(|stage| stage.stage_name == "selection.final")
		{
			stage.items = final_stage_items;
		}

		for stage in trajectory_stages {
			trace_builder.push_stage(stage);
		}

		(items, trace_builder.build())
	}

	async fn write_trace_payload(&self, trace_id: Uuid, trace_payload: TracePayload) -> Result<()> {
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

		Ok(())
	}

	async fn build_snippet_items(
		&self,
		filtered_candidates: &[ChunkCandidate],
		note_meta: &HashMap<Uuid, NoteMeta>,
	) -> Result<Vec<ChunkSnippet>> {
		if filtered_candidates.is_empty() {
			return Ok(Vec::new());
		}

		let pairs = ranking::collect_neighbor_pairs(filtered_candidates);
		let chunk_rows = fetch_chunks_by_pair(&self.db.pool, &pairs).await?;
		let mut chunk_by_id = HashMap::new();
		let mut chunk_by_note_index = HashMap::new();

		for row in chunk_rows {
			chunk_by_note_index.insert((row.note_id, row.chunk_index), row.clone());
			chunk_by_id.insert(row.chunk_id, row);
		}

		let mut items = Vec::new();

		for candidate in filtered_candidates {
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

		Ok(items)
	}

	async fn rerank_snippet_items(
		&self,
		query: &str,
		snippet_items: &[ChunkSnippet],
		cache_cfg: &SearchCache,
		now: OffsetDateTime,
	) -> Result<Vec<f32>> {
		if snippet_items.is_empty() {
			return Ok(Vec::new());
		}

		let (cache_candidates, signature) = Self::build_rerank_cache_signature(snippet_items);
		let mut cache_key: Option<String> = None;
		let mut cached_scores: Option<Vec<f32>> = None;

		if cache_cfg.enabled {
			match ranking::build_rerank_cache_key(
				query,
				self.cfg.providers.rerank.provider_id.as_str(),
				self.cfg.providers.rerank.model.as_str(),
				&signature,
			) {
				Ok(key) => {
					cache_key = Some(key.clone());
					cached_scores = self
						.read_rerank_cache_scores(&key, cache_candidates.as_slice(), cache_cfg, now)
						.await;
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

		if let Some(scores) = cached_scores {
			return Ok(scores);
		}

		let docs: Vec<String> = snippet_items.iter().map(|item| item.snippet.clone()).collect();
		let scores = self.providers.rerank.rerank(&self.cfg.providers.rerank, query, &docs).await?;

		if scores.len() != snippet_items.len() {
			return Err(Error::Provider {
				message: "Rerank provider returned mismatched score count.".to_string(),
			});
		}
		if cache_cfg.enabled
			&& let Some(key) = cache_key.as_ref()
			&& !cache_candidates.is_empty()
		{
			self.store_rerank_cache_scores(
				key,
				cache_candidates.as_slice(),
				scores.as_slice(),
				cache_cfg,
			)
			.await;
		}

		Ok(scores)
	}

	fn build_rerank_cache_signature(
		snippet_items: &[ChunkSnippet],
	) -> (Vec<RerankCacheCandidate>, Vec<(Uuid, OffsetDateTime)>) {
		let candidates: Vec<RerankCacheCandidate> = snippet_items
			.iter()
			.map(|item| RerankCacheCandidate {
				chunk_id: item.chunk.chunk_id,
				updated_at: item.note.updated_at,
			})
			.collect();
		let signature: Vec<(Uuid, OffsetDateTime)> =
			candidates.iter().map(|candidate| (candidate.chunk_id, candidate.updated_at)).collect();

		(candidates, signature)
	}

	async fn read_rerank_cache_scores(
		&self,
		key: &str,
		cache_candidates: &[RerankCacheCandidate],
		cache_cfg: &SearchCache,
		now: OffsetDateTime,
	) -> Option<Vec<f32>> {
		match fetch_cache_payload(&self.db.pool, CacheKind::Rerank, key, now).await {
			Ok(Some(payload)) => {
				let decoded: RerankCachePayload = match serde_json::from_value(payload.value) {
					Ok(value) => value,
					Err(err) => {
						tracing::warn!(
							error = %err,
							cache_kind = CacheKind::Rerank.as_str(),
							cache_key_prefix = ranking::cache_key_prefix(key),
							"Cache payload decode failed."
						);

						RerankCachePayload { items: Vec::new() }
					},
				};

				if let Some(scores) = ranking::build_cached_scores(&decoded, cache_candidates) {
					tracing::info!(
						cache_kind = CacheKind::Rerank.as_str(),
						cache_key_prefix = ranking::cache_key_prefix(key),
						hit = true,
						payload_size = payload.size_bytes,
						ttl_days = cache_cfg.rerank_ttl_days,
						"Cache hit."
					);

					Some(scores)
				} else {
					tracing::warn!(
						cache_kind = CacheKind::Rerank.as_str(),
						cache_key_prefix = ranking::cache_key_prefix(key),
						hit = false,
						payload_size = payload.size_bytes,
						ttl_days = cache_cfg.rerank_ttl_days,
						"Cache payload did not match candidates."
					);

					None
				}
			},
			Ok(None) => {
				tracing::info!(
					cache_kind = CacheKind::Rerank.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					hit = false,
					payload_size = 0_u64,
					ttl_days = cache_cfg.rerank_ttl_days,
					"Cache miss."
				);

				None
			},
			Err(err) => {
				tracing::warn!(
					error = %err,
					cache_kind = CacheKind::Rerank.as_str(),
					cache_key_prefix = ranking::cache_key_prefix(key),
					"Cache read failed."
				);

				None
			},
		}
	}

	async fn store_rerank_cache_scores(
		&self,
		key: &str,
		cache_candidates: &[RerankCacheCandidate],
		scores: &[f32],
		cache_cfg: &SearchCache,
	) {
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

	async fn fetch_note_meta_for_candidates(
		&self,
		tenant_id: &str,
		project_id: &str,
		agent_id: &str,
		allowed_scopes: &[String],
		candidate_note_ids: &[Uuid],
		now: OffsetDateTime,
	) -> Result<HashMap<Uuid, NoteMeta>> {
		if candidate_note_ids.is_empty() {
			return Ok(HashMap::new());
		}

		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			org_shared_allowed,
		)
		.await?;
		let notes: Vec<MemoryNote> = sqlx::query_as(
			"\
SELECT *
FROM memory_notes
WHERE note_id = ANY($1::uuid[])
  AND tenant_id = $2
  AND (
    project_id = $3
    OR (project_id = $4 AND scope = 'org_shared')
  )",
		)
		.bind(candidate_note_ids)
		.bind(tenant_id)
		.bind(project_id)
		.bind(access::ORG_PROJECT_ID)
		.fetch_all(&self.db.pool)
		.await?;
		let mut note_meta = HashMap::new();

		for note in notes {
			if !access::note_read_allowed(&note, agent_id, allowed_scopes, &shared_grants, now) {
				continue;
			}

			note_meta.insert(
				note.note_id,
				NoteMeta {
					note_id: note.note_id,
					note_type: note.r#type,
					key: note.key,
					scope: note.scope,
					agent_id: note.agent_id,
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

		Ok(note_meta)
	}

	async fn fetch_relation_contexts_for_notes(
		&self,
		note_ids: &[Uuid],
		tenant_id: &str,
		project_id: &str,
		agent_id: &str,
		allowed_scopes: &[String],
		now: OffsetDateTime,
	) -> Result<HashMap<Uuid, Vec<SearchExplainRelationContext>>> {
		if note_ids.is_empty() {
			return Ok(HashMap::new());
		}

		let private_allowed = allowed_scopes.iter().any(|scope| scope == "agent_private");
		let non_private_scopes: Vec<String> =
			allowed_scopes.iter().filter(|scope| *scope != "agent_private").cloned().collect();
		let (max_evidence_notes_per_fact, max_facts_per_item) = self.relation_context_bounds();
		let rows = self
			.fetch_relation_context_rows(
				note_ids,
				tenant_id,
				project_id,
				agent_id,
				&non_private_scopes,
				private_allowed,
				now,
				max_evidence_notes_per_fact,
				max_facts_per_item,
			)
			.await?;

		Ok(Self::group_relation_context_rows(rows))
	}

	fn relation_context_bounds(&self) -> (i32, i32) {
		let max_evidence_notes_per_fact =
			i32::try_from(self.cfg.search.graph_context.max_evidence_notes_per_fact)
				.unwrap_or(i32::MAX);
		let max_facts_per_item =
			i32::try_from(self.cfg.search.graph_context.max_facts_per_item).unwrap_or(i32::MAX);

		(max_evidence_notes_per_fact, max_facts_per_item)
	}

	#[allow(clippy::too_many_arguments)]
	async fn fetch_relation_context_rows(
		&self,
		note_ids: &[Uuid],
		tenant_id: &str,
		project_id: &str,
		agent_id: &str,
		non_private_scopes: &[String],
		private_allowed: bool,
		now: OffsetDateTime,
		max_evidence_notes_per_fact: i32,
		max_facts_per_item: i32,
	) -> Result<Vec<SearchRelationContextRow>> {
		Ok(sqlx::query_as::<_, SearchRelationContextRow>(RELATION_CONTEXT_SQL)
			.bind(tenant_id)
			.bind(project_id)
			.bind(agent_id)
			.bind(now)
			.bind(private_allowed)
			.bind(non_private_scopes)
			.bind(note_ids)
			.bind(max_evidence_notes_per_fact)
			.bind(max_facts_per_item)
			.fetch_all(&self.db.pool)
			.await?)
	}

	fn group_relation_context_rows(
		rows: Vec<SearchRelationContextRow>,
	) -> HashMap<Uuid, Vec<SearchExplainRelationContext>> {
		let mut relation_context_by_note: HashMap<Uuid, Vec<SearchExplainRelationContext>> =
			HashMap::new();

		for row in rows {
			let object = if row.object_entity_id.is_some() {
				SearchExplainRelationContextObject {
					entity: Some(SearchExplainRelationEntityRef {
						canonical: row.object_canonical,
						kind: row.object_kind,
					}),
					value: None,
				}
			} else {
				SearchExplainRelationContextObject { entity: None, value: row.object_value }
			};

			relation_context_by_note.entry(row.note_id).or_default().push(
				SearchExplainRelationContext {
					fact_id: row.fact_id,
					scope: row.scope,
					subject: SearchExplainRelationEntityRef {
						canonical: row.subject_canonical,
						kind: row.subject_kind,
					},
					predicate: row.predicate,
					object,
					valid_from: row.valid_from,
					valid_to: row.valid_to,
					evidence_note_ids: row.evidence_note_ids,
				},
			);
		}

		relation_context_by_note
	}
}

pub(crate) fn resolve_read_profile_scopes(cfg: &Config, profile: &str) -> Result<Vec<String>> {
	ranking::resolve_scopes(cfg, profile)
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
	let query_tokens = ranking::tokenize_query(trace.query.as_str(), MAX_MATCHED_TERMS);
	let scope_context_boost_by_scope =
		ranking::build_scope_context_boost_by_scope(&query_tokens, cfg.context.as_ref());
	let det_query_tokens = build_deterministic_query_tokens(cfg, trace.query.as_str());
	let blend_policy = ranking::resolve_blend_policy(
		&cfg.ranking.blend,
		ranking_override.and_then(|override_| override_.blend.as_ref()),
	)?;
	let diversity_policy = ranking::resolve_diversity_policy(
		&cfg.ranking.diversity,
		ranking_override.and_then(|override_| override_.diversity.as_ref()),
	)?;
	let policy_id = ranking_policy_id(cfg, ranking_override)?;
	let now = trace.created_at;
	let total_rerank = u32::try_from(candidates.len()).unwrap_or(1).max(1);
	let total_retrieval = trace.candidate_count.max(1);
	let rerank_ranks = ranking::build_rerank_ranks_for_replay(candidates);
	let replay_diversity_decisions = ranking::extract_replay_diversity_decisions(candidates);
	let score_ctx = ScoreCandidateCtx {
		cfg,
		blend_policy: &blend_policy,
		scope_context_boost_by_scope: &scope_context_boost_by_scope,
		det_query_tokens: det_query_tokens.as_slice(),
		now,
		total_rerank,
		total_retrieval,
	};
	let mut best_by_note: BTreeMap<Uuid, ScoredReplay> = BTreeMap::new();

	for (candidate, rerank_rank) in candidates.iter().zip(rerank_ranks) {
		let scored = score_replay_candidate(&score_ctx, candidate, rerank_rank);
		let replace = match best_by_note.get(&candidate.note_id) {
			None => true,
			Some(existing) => should_replace_replay_best(existing, &scored),
		};

		if replace {
			best_by_note.insert(candidate.note_id, scored);
		}
	}

	let mut results: Vec<ScoredReplay> = best_by_note.into_values().collect();

	results.sort_by(cmp_scored_replay);

	let results = apply_replay_diversity_selection(
		results,
		top_k,
		diversity_policy.enabled,
		&replay_diversity_decisions,
	);

	Ok(build_replay_items(
		cfg,
		&blend_policy,
		&diversity_policy,
		policy_id.as_str(),
		&replay_diversity_decisions,
		results,
	))
}

fn validate_search_request_inputs(
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	query: &str,
) -> Result<()> {
	if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
		return Err(Error::InvalidRequest {
			message: "tenant_id, project_id, and agent_id are required.".to_string(),
		});
	}
	if !elf_domain::english_gate::is_english_natural_language(query) {
		return Err(Error::NonEnglishInput { field: "$.query".to_string() });
	}

	Ok(())
}

fn raw_search_path_label(path: RawSearchPath) -> &'static str {
	match path {
		RawSearchPath::Quick => "quick",
		RawSearchPath::Planned => "planned",
	}
}

fn sorted_unique_strings(mut values: Vec<String>) -> Vec<String> {
	values.sort();
	values.dedup();

	values
}

fn build_trajectory_summary_from_stages(
	stages: &[SearchTrajectoryStage],
) -> SearchTrajectorySummary {
	let summary_stages = stages
		.iter()
		.map(|stage| {
			let stats =
				stage.stage_payload.get("stats").cloned().unwrap_or_else(|| serde_json::json!({}));

			SearchTrajectorySummaryStage {
				stage_order: stage.stage_order,
				stage_name: stage.stage_name.clone(),
				item_count: stage.items.len() as u32,
				stats,
			}
		})
		.collect();

	SearchTrajectorySummary {
		schema: SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1.to_string(),
		stages: summary_stages,
	}
}

fn build_search_filter(
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	allowed_scopes: &[String],
) -> Filter {
	let private_scope = "agent_private".to_string();
	let non_private_scopes: Vec<String> =
		allowed_scopes.iter().filter(|scope| *scope != "agent_private").cloned().collect();
	let mut scope_should_conditions = Vec::new();

	if allowed_scopes.iter().any(|scope| scope == "agent_private") {
		let private_filter = Filter::all([
			Condition::matches("scope", private_scope),
			Condition::matches("agent_id", agent_id.to_string()),
		]);

		scope_should_conditions.push(Condition::from(private_filter));
	}
	if !non_private_scopes.is_empty() {
		scope_should_conditions.push(Condition::matches("scope", non_private_scopes));
	}

	let scope_min_should = if scope_should_conditions.is_empty() {
		None
	} else {
		Some(MinShould { min_count: 1, conditions: scope_should_conditions })
	};
	let mut project_or_org_branches = vec![Condition::from(Filter {
		must: vec![Condition::matches("project_id", project_id.to_string())],
		should: Vec::new(),
		must_not: Vec::new(),
		min_should: scope_min_should,
	})];

	if allowed_scopes.iter().any(|scope| scope == "org_shared") {
		let org_filter = Filter::all([
			Condition::matches("project_id", access::ORG_PROJECT_ID.to_string()),
			Condition::matches("scope", "org_shared".to_string()),
		]);

		project_or_org_branches.push(Condition::from(org_filter));
	}

	Filter {
		must: vec![
			Condition::matches("tenant_id", tenant_id.to_string()),
			Condition::matches("status", "active".to_string()),
		],
		should: Vec::new(),
		must_not: Vec::new(),
		min_should: Some(MinShould { min_count: 1, conditions: project_or_org_branches }),
	}
}

fn select_best_scored_chunks(scored: Vec<ScoredChunk>) -> Vec<ScoredChunk> {
	let mut best_by_note: HashMap<Uuid, ScoredChunk> = HashMap::new();

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

	results.sort_by(cmp_scored_chunk);

	results
}

fn cmp_scored_chunk(a: &ScoredChunk, b: &ScoredChunk) -> Ordering {
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
}

fn score_chunk_candidate(
	ctx: &ScoreCandidateCtx<'_, '_>,
	item: ChunkSnippet,
	rerank_score: f32,
	rerank_rank: u32,
) -> ScoredChunk {
	let importance = item.note.importance;
	let retrieval_rank = item.retrieval_rank;
	let age_days = (ctx.now - item.note.updated_at).as_seconds_f32() / 86_400.0;
	let decay = if ctx.cfg.ranking.recency_tau_days > 0.0 {
		(-age_days / ctx.cfg.ranking.recency_tau_days).exp()
	} else {
		1.0
	};
	let base = (1.0 + 0.6 * importance) * decay;
	let tie_breaker_score = ctx.cfg.ranking.tie_breaker_weight * base;
	let scope_context_boost =
		ctx.scope_context_boost_by_scope.get(item.note.scope.as_str()).copied().unwrap_or(0.0);
	let rerank_norm = match ctx.blend_policy.rerank_normalization {
		ranking::NormalizationKind::Rank => ranking::rank_normalize(rerank_rank, ctx.total_rerank),
	};
	let retrieval_norm = match ctx.blend_policy.retrieval_normalization {
		ranking::NormalizationKind::Rank =>
			ranking::rank_normalize(retrieval_rank, ctx.total_retrieval),
	};
	let blend_retrieval_weight = if ctx.blend_policy.enabled {
		ranking::retrieval_weight_for_rank(retrieval_rank, &ctx.blend_policy.segments)
	} else {
		0.0
	};
	let retrieval_term = blend_retrieval_weight * retrieval_norm;
	let rerank_term = (1.0 - blend_retrieval_weight) * rerank_norm;
	let det_terms = ranking::compute_deterministic_ranking_terms(
		ctx.cfg,
		ctx.det_query_tokens,
		item.snippet.as_str(),
		item.note.hit_count,
		item.note.last_hit_at,
		age_days,
		ctx.now,
	);
	let final_score = retrieval_term
		+ rerank_term
		+ tie_breaker_score
		+ scope_context_boost
		+ det_terms.lexical_bonus
		+ det_terms.hit_boost
		+ det_terms.decay_penalty;

	ScoredChunk {
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
	}
}

fn build_trace_candidate_record(
	scored_chunk: &ScoredChunk,
	now: OffsetDateTime,
	expires_at: OffsetDateTime,
) -> TraceCandidateRecord {
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
		expires_at,
	}
}

fn build_search_item_and_trace_item(
	args: BuildSearchItemArgs<'_>,
) -> (SearchItem, TraceItemRecord) {
	let (matched_terms, matched_fields) = ranking::match_terms_in_text(
		args.query_tokens,
		args.scored_chunk.item.snippet.as_str(),
		args.scored_chunk.item.note.key.as_deref(),
		MAX_MATCHED_TERMS,
	);
	let matched_fields = ranking::merge_matched_fields(
		matched_fields,
		args.structured_matches.get(&args.scored_chunk.item.note.note_id),
	);
	let trace_terms =
		ranking_explain_v2::build_trace_terms_v2(ranking_explain_v2::TraceTermsArgs {
			cfg: args.cfg,
			blend_enabled: args.blend_policy.enabled,
			retrieval_normalization: args.blend_policy.retrieval_normalization.as_str(),
			rerank_normalization: args.blend_policy.rerank_normalization.as_str(),
			blend_retrieval_weight: args.scored_chunk.blend_retrieval_weight,
			retrieval_rank: args.scored_chunk.item.retrieval_rank,
			retrieval_norm: args.scored_chunk.retrieval_norm,
			retrieval_term: args.scored_chunk.retrieval_term,
			rerank_score: args.scored_chunk.rerank_score,
			rerank_rank: args.scored_chunk.rerank_rank,
			rerank_norm: args.scored_chunk.rerank_norm,
			rerank_term: args.scored_chunk.rerank_term,
			tie_breaker_score: args.scored_chunk.tie_breaker_score,
			importance: args.scored_chunk.importance,
			age_days: args.scored_chunk.age_days,
			scope: args.scored_chunk.item.note.scope.as_str(),
			scope_context_boost: args.scored_chunk.scope_context_boost,
			deterministic_lexical_overlap_ratio: args
				.scored_chunk
				.deterministic_lexical_overlap_ratio,
			deterministic_lexical_bonus: args.scored_chunk.deterministic_lexical_bonus,
			deterministic_hit_count: args.scored_chunk.deterministic_hit_count,
			deterministic_last_hit_age_days: args.scored_chunk.deterministic_last_hit_age_days,
			deterministic_hit_boost: args.scored_chunk.deterministic_hit_boost,
			deterministic_decay_penalty: args.scored_chunk.deterministic_decay_penalty,
		});
	let response_terms = ranking_explain_v2::strip_term_inputs(&trace_terms);
	let relation_context =
		args.relation_contexts.get(&args.scored_chunk.item.note.note_id).cloned();
	let diversity = if args.diversity_policy.enabled {
		args.diversity_decisions
			.get(&args.scored_chunk.item.note.note_id)
			.map(ranking::build_diversity_explain)
	} else {
		None
	};
	let response_explain = SearchExplain {
		r#match: SearchMatchExplain {
			matched_terms: matched_terms.clone(),
			matched_fields: matched_fields.clone(),
		},
		ranking: SearchRankingExplain {
			schema: ranking_explain_v2::SEARCH_RANKING_EXPLAIN_SCHEMA_V2.to_string(),
			policy_id: args.policy_id.to_string(),
			final_score: args.scored_chunk.final_score,
			terms: response_terms,
		},
		relation_context: relation_context.clone(),
		diversity: diversity.clone(),
	};
	let trace_explain = SearchExplain {
		r#match: SearchMatchExplain { matched_terms, matched_fields },
		ranking: SearchRankingExplain {
			schema: ranking_explain_v2::SEARCH_RANKING_EXPLAIN_SCHEMA_V2.to_string(),
			policy_id: args.policy_id.to_string(),
			final_score: args.scored_chunk.final_score,
			terms: trace_terms,
		},
		relation_context,
		diversity,
	};
	let result_handle = Uuid::new_v4();
	let note = &args.scored_chunk.item.note;
	let chunk = &args.scored_chunk.item.chunk;
	let item = SearchItem {
		result_handle,
		note_id: note.note_id,
		chunk_id: chunk.chunk_id,
		chunk_index: chunk.chunk_index,
		start_offset: chunk.start_offset,
		end_offset: chunk.end_offset,
		snippet: args.scored_chunk.item.snippet.clone(),
		r#type: note.note_type.clone(),
		key: note.key.clone(),
		scope: note.scope.clone(),
		importance: note.importance,
		confidence: note.confidence,
		updated_at: note.updated_at,
		expires_at: note.expires_at,
		final_score: args.scored_chunk.final_score,
		source_ref: note.source_ref.clone(),
		explain: response_explain,
	};
	let trace_item = TraceItemRecord {
		item_id: result_handle,
		note_id: note.note_id,
		chunk_id: Some(chunk.chunk_id),
		rank: args.rank,
		final_score: args.scored_chunk.final_score,
		explain: trace_explain,
	};

	(item, trace_item)
}

fn build_structured_field_matches(rows: Vec<FieldHit>) -> (Vec<Uuid>, HashMap<Uuid, Vec<String>>) {
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

	(ordered_note_ids, structured_matches_out)
}

fn build_structured_field_candidates(
	candidate_k: u32,
	ordered_note_ids: Vec<Uuid>,
	best_by_note: HashMap<Uuid, (Uuid, i32)>,
	embed_version: &str,
) -> Vec<ChunkCandidate> {
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
			scope: None,
			updated_at: None,
			embedding_version: Some(embed_version.to_string()),
		});

		next_rank = next_rank.saturating_add(1);
	}

	structured_candidates
}

fn build_deterministic_query_tokens(cfg: &Config, query: &str) -> Vec<String> {
	if cfg.ranking.deterministic.enabled
		&& cfg.ranking.deterministic.lexical.enabled
		&& cfg.ranking.deterministic.lexical.max_query_terms > 0
	{
		ranking::tokenize_query(query, cfg.ranking.deterministic.lexical.max_query_terms as usize)
	} else {
		Vec::new()
	}
}

fn build_trace_audit(actor_id: &str, token_id: Option<&str>) -> Value {
	match token_id.map(str::trim).filter(|value| !value.is_empty()) {
		Some(token_id) => serde_json::json!({ "actor_id": actor_id, "token_id": token_id }),
		None => serde_json::json!({ "actor_id": actor_id }),
	}
}

fn build_trace_trajectory_stages(args: &BuildTraceArgs<'_>) -> Vec<TraceTrajectoryStageRecord> {
	let path_label = raw_search_path_label(args.path);

	vec![
		build_trace_rewrite_stage(args, path_label),
		build_trace_recall_stage(args, path_label),
		build_trace_fusion_stage(args, path_label),
		build_trace_rerank_stage(args, path_label),
		build_trace_final_stage(args, path_label),
	]
}

fn build_trace_rewrite_stage(
	args: &BuildTraceArgs<'_>,
	path_label: &str,
) -> TraceTrajectoryStageRecord {
	let expanded_queries = sorted_unique_strings(args.expanded_queries.clone());

	TraceTrajectoryStageRecord {
		stage_id: Uuid::new_v4(),
		stage_order: 1,
		stage_name: "rewrite.expansion".to_string(),
		stage_payload: serde_json::json!({
			"schema": SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1,
			"path": path_label,
			"inputs": {
				"query": args.query,
				"expansion_mode": ranking::expansion_mode_label(args.expansion_mode),
			},
			"outputs": {
				"expanded_queries": expanded_queries,
			},
			"stats": {
				"expanded_query_count": args.expanded_queries.len(),
			},
		}),
		created_at: args.now,
		items: Vec::new(),
	}
}

fn build_trace_recall_stage(
	args: &BuildTraceArgs<'_>,
	path_label: &str,
) -> TraceTrajectoryStageRecord {
	let mut stage_payload = serde_json::json!({
		"schema": SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1,
		"path": path_label,
		"stats": {
			"candidate_count_before_filter": args.candidate_count,
			"candidate_count_after_filter": args.filtered_candidate_count,
			"snippet_count": args.snippet_count,
		},
	});

	if let Some(filter_impact) = &args.filter_impact
		&& let Some(payload) = stage_payload.as_object_mut()
	{
		payload.insert("filter_impact".to_string(), filter_impact.to_stage_payload());
	}
	if let Some(recursive_retrieval) = args.recursive_retrieval
		&& recursive_retrieval.enabled
		&& let Some(payload) = stage_payload.as_object_mut()
	{
		payload.insert(
			"recursive".to_string(),
			serde_json::json!({
				"enabled": true,
				"scopes_seeded": recursive_retrieval.scopes_seeded,
				"scopes_queried": recursive_retrieval.scopes_queried,
				"candidates_before": recursive_retrieval.candidates_before,
				"candidates_added": recursive_retrieval.candidates_added,
				"candidates_after": recursive_retrieval.candidates_after,
				"rounds_executed": recursive_retrieval.rounds_executed,
				"total_queries": recursive_retrieval.total_queries,
				"stop_reason": recursive_retrieval
					.stop_reason
					.clone()
					.unwrap_or_else(|| "converged".to_string()),
			}),
		);
	}

	let items: Vec<TraceTrajectoryStageItemRecord> = args
		.recall_candidates
		.iter()
		.take(MAX_TRAJECTORY_STAGE_ITEMS)
		.map(|candidate| TraceTrajectoryStageItemRecord {
			id: Uuid::new_v4(),
			item_id: None,
			note_id: Some(candidate.note_id),
			chunk_id: Some(candidate.chunk_id),
			metrics: serde_json::json!({
				"retrieval_rank": candidate.retrieval_rank,
				"chunk_index": candidate.chunk_index,
			}),
		})
		.collect();

	TraceTrajectoryStageRecord {
		stage_id: Uuid::new_v4(),
		stage_order: 2,
		stage_name: "recall.candidates".to_string(),
		stage_payload,
		created_at: args.now,
		items,
	}
}

fn build_trace_fusion_stage(
	args: &BuildTraceArgs<'_>,
	path_label: &str,
) -> TraceTrajectoryStageRecord {
	let items: Vec<TraceTrajectoryStageItemRecord> = args
		.fused_results
		.iter()
		.take(MAX_TRAJECTORY_STAGE_ITEMS)
		.map(|scored| TraceTrajectoryStageItemRecord {
			id: Uuid::new_v4(),
			item_id: None,
			note_id: Some(scored.item.note.note_id),
			chunk_id: Some(scored.item.chunk.chunk_id),
			metrics: serde_json::json!({
				"retrieval_rank": scored.item.retrieval_rank,
				"final_score": scored.final_score,
			}),
		})
		.collect();

	TraceTrajectoryStageRecord {
		stage_id: Uuid::new_v4(),
		stage_order: 3,
		stage_name: "fusion.merge".to_string(),
		stage_payload: serde_json::json!({
			"schema": SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1,
			"path": path_label,
			"stats": {
				"scored_count": args.scored_count,
				"fused_count": args.fused_count,
			},
			"decisions": {
				"fusion_weight": args.policies.retrieval_sources_policy.fusion_weight,
				"structured_field_weight": args.policies.retrieval_sources_policy.structured_field_weight,
				"fusion_priority": args.policies.retrieval_sources_policy.fusion_priority,
				"structured_field_priority": args.policies.retrieval_sources_policy.structured_field_priority,
			},
		}),
		created_at: args.now,
		items,
	}
}

fn build_trace_rerank_stage(
	args: &BuildTraceArgs<'_>,
	path_label: &str,
) -> TraceTrajectoryStageRecord {
	let items: Vec<TraceTrajectoryStageItemRecord> = args
		.fused_results
		.iter()
		.take(MAX_TRAJECTORY_STAGE_ITEMS)
		.map(|scored| TraceTrajectoryStageItemRecord {
			id: Uuid::new_v4(),
			item_id: None,
			note_id: Some(scored.item.note.note_id),
			chunk_id: Some(scored.item.chunk.chunk_id),
			metrics: serde_json::json!({
				"rerank_score": scored.rerank_score,
				"rerank_rank": scored.rerank_rank,
				"rerank_norm": scored.rerank_norm,
				"retrieval_norm": scored.retrieval_norm,
				"final_score": scored.final_score,
			}),
		})
		.collect();

	TraceTrajectoryStageRecord {
		stage_id: Uuid::new_v4(),
		stage_order: 4,
		stage_name: "rerank.score".to_string(),
		stage_payload: serde_json::json!({
			"schema": SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1,
			"path": path_label,
			"stats": {
				"reranked_count": args.scored_count,
			},
			"decisions": {
				"blend_enabled": args.policies.blend_policy.enabled,
				"diversity_enabled": args.policies.diversity_policy.enabled,
			},
		}),
		created_at: args.now,
		items,
	}
}

fn build_trace_final_stage(
	args: &BuildTraceArgs<'_>,
	path_label: &str,
) -> TraceTrajectoryStageRecord {
	TraceTrajectoryStageRecord {
		stage_id: Uuid::new_v4(),
		stage_order: 5,
		stage_name: "selection.final".to_string(),
		stage_payload: serde_json::json!({
			"schema": SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1,
			"path": path_label,
			"stats": {
				"selected_count": args.selected_count,
				"top_k": args.top_k,
			},
		}),
		created_at: args.now,
		items: Vec::new(),
	}
}

fn score_replay_candidate(
	ctx: &ScoreCandidateCtx<'_, '_>,
	candidate: &TraceReplayCandidate,
	rerank_rank: u32,
) -> ScoredReplay {
	let importance = candidate.note_importance;
	let retrieval_rank = candidate.retrieval_rank;
	let age_days = (ctx.now - candidate.note_updated_at).as_seconds_f32() / 86_400.0;
	let decay = if ctx.cfg.ranking.recency_tau_days > 0.0 {
		(-age_days / ctx.cfg.ranking.recency_tau_days).exp()
	} else {
		1.0
	};
	let base = (1.0 + 0.6 * importance) * decay;
	let tie_breaker_score = ctx.cfg.ranking.tie_breaker_weight * base;
	let scope_context_boost =
		ctx.scope_context_boost_by_scope.get(candidate.note_scope.as_str()).copied().unwrap_or(0.0);
	let rerank_norm = match ctx.blend_policy.rerank_normalization {
		ranking::NormalizationKind::Rank => ranking::rank_normalize(rerank_rank, ctx.total_rerank),
	};
	let retrieval_norm = match ctx.blend_policy.retrieval_normalization {
		ranking::NormalizationKind::Rank =>
			ranking::rank_normalize(retrieval_rank, ctx.total_retrieval),
	};
	let blend_retrieval_weight = if ctx.blend_policy.enabled {
		ranking::retrieval_weight_for_rank(retrieval_rank, &ctx.blend_policy.segments)
	} else {
		0.0
	};
	let retrieval_term = blend_retrieval_weight * retrieval_norm;
	let rerank_term = (1.0 - blend_retrieval_weight) * rerank_norm;
	let det_terms = ranking::compute_deterministic_ranking_terms(
		ctx.cfg,
		ctx.det_query_tokens,
		candidate.snippet.as_str(),
		candidate.note_hit_count,
		candidate.note_last_hit_at,
		age_days,
		ctx.now,
	);
	let final_score = retrieval_term
		+ rerank_term
		+ tie_breaker_score
		+ scope_context_boost
		+ det_terms.lexical_bonus
		+ det_terms.hit_boost
		+ det_terms.decay_penalty;

	ScoredReplay {
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
	}
}

fn should_replace_replay_best(existing: &ScoredReplay, scored: &ScoredReplay) -> bool {
	let ord = ranking::cmp_f32_desc(scored.final_score, existing.final_score);

	if ord != Ordering::Equal {
		ord == Ordering::Less
	} else {
		scored.retrieval_rank < existing.retrieval_rank
	}
}

fn cmp_scored_replay(a: &ScoredReplay, b: &ScoredReplay) -> Ordering {
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
}

fn apply_replay_diversity_selection(
	mut results: Vec<ScoredReplay>,
	top_k: u32,
	diversity_enabled: bool,
	replay_diversity_decisions: &HashMap<Uuid, DiversityDecision>,
) -> Vec<ScoredReplay> {
	if diversity_enabled && !replay_diversity_decisions.is_empty() {
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

	results
}

fn build_replay_items(
	cfg: &Config,
	blend_policy: &ResolvedBlendPolicy,
	diversity_policy: &ResolvedDiversityPolicy,
	policy_id: &str,
	replay_diversity_decisions: &HashMap<Uuid, DiversityDecision>,
	results: Vec<ScoredReplay>,
) -> Vec<TraceReplayItem> {
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
				policy_id: policy_id.to_string(),
				final_score: scored.final_score,
				terms,
			},
			relation_context: None,
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

	out
}

async fn load_trace_trajectory_summary(
	pool: &PgPool,
	trace_id: Uuid,
) -> Result<Option<SearchTrajectorySummary>> {
	let stages = load_trace_trajectory_stages(pool, trace_id).await?;

	if stages.is_empty() {
		Ok(None)
	} else {
		Ok(Some(build_trajectory_summary_from_stages(stages.as_slice())))
	}
}

async fn load_trace_trajectory_stages(
	pool: &PgPool,
	trace_id: Uuid,
) -> Result<Vec<SearchTrajectoryStage>> {
	let rows = sqlx::query(
		"\
	SELECT
	s.stage_id,
	s.stage_order,
	s.stage_name,
	s.stage_payload,
	i.item_id,
	i.note_id,
	i.chunk_id,
	i.metrics
FROM search_trace_stages s
LEFT JOIN search_trace_stage_items i ON i.stage_id = s.stage_id
WHERE s.trace_id = $1
ORDER BY s.stage_order ASC, i.item_id ASC NULLS LAST, i.note_id ASC NULLS LAST",
	)
	.bind(trace_id)
	.fetch_all(pool)
	.await?;
	let mut stages = Vec::new();
	let mut stage_pos_by_id: HashMap<Uuid, usize> = HashMap::new();

	for row in rows {
		let stage_id: Uuid = row.try_get("stage_id")?;
		let idx = if let Some(idx) = stage_pos_by_id.get(&stage_id).copied() {
			idx
		} else {
			let stage_order: i32 = row.try_get("stage_order")?;
			let stage_name: String = row.try_get("stage_name")?;
			let stage_payload: Value = row.try_get("stage_payload")?;
			let idx = stages.len();

			stages.push(SearchTrajectoryStage {
				stage_order: stage_order as u32,
				stage_name,
				stage_payload,
				items: Vec::new(),
			});
			stage_pos_by_id.insert(stage_id, idx);

			idx
		};
		let item_metrics: Option<Value> = row.try_get("metrics")?;

		if let Some(metrics) = item_metrics {
			stages[idx].items.push(SearchTrajectoryStageItem {
				item_id: row.try_get("item_id")?,
				note_id: row.try_get("note_id")?,
				chunk_id: row.try_get("chunk_id")?,
				metrics,
			});
		}
	}

	Ok(stages)
}

async fn load_item_trajectory(
	pool: &PgPool,
	trace_id: Uuid,
	item_id: Uuid,
) -> Result<Option<SearchExplainTrajectory>> {
	let rows = sqlx::query(
		"\
SELECT
	s.stage_order,
	s.stage_name,
	i.metrics
FROM search_trace_stages s
JOIN search_trace_stage_items i ON i.stage_id = s.stage_id
WHERE s.trace_id = $1 AND i.item_id = $2
ORDER BY s.stage_order ASC",
	)
	.bind(trace_id)
	.bind(item_id)
	.fetch_all(pool)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}

	let mut stages = Vec::with_capacity(rows.len());

	for row in rows {
		let stage_order: i32 = row.try_get("stage_order")?;
		let stage_name: String = row.try_get("stage_name")?;
		let metrics: Value = row.try_get("metrics")?;

		stages.push(SearchExplainTrajectoryStage {
			stage_order: stage_order as u32,
			stage_name,
			metrics,
		});
	}

	Ok(Some(SearchExplainTrajectory {
		schema: SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1.to_string(),
		stages,
	}))
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
	e.note_id,
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

	sqlx::query(
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
	)
	.bind(Uuid::new_v4())
	.bind(payload.trace.trace_id)
	.bind(now)
	.bind(payload_json)
	.execute(executor)
	.await?;

	Ok(())
}

async fn persist_trace_inline(executor: &mut PgConnection, payload: TracePayload) -> Result<()> {
	let trace = payload.trace;
	let items = payload.items;
	let candidates = payload.candidates;
	let stages = payload.stages;
	let trace_id = trace.trace_id;

	persist_trace_inline_header(executor, &trace).await?;
	persist_trace_inline_items(executor, trace_id, items).await?;
	persist_trace_inline_stages(executor, trace_id, stages).await?;
	persist_trace_inline_candidates(executor, trace_id, candidates).await?;

	Ok(())
}

async fn persist_trace_inline_stages(
	executor: &mut PgConnection,
	trace_id: Uuid,
	stages: Vec<TraceTrajectoryStageRecord>,
) -> Result<()> {
	if stages.is_empty() {
		return Ok(());
	}

	let mut item_records = Vec::new();
	let mut stage_builder = QueryBuilder::new(
		"\
INSERT INTO search_trace_stages (
	stage_id,
	trace_id,
	stage_order,
	stage_name,
	stage_payload,
	created_at
) ",
	);

	stage_builder.push_values(stages, |mut b, stage| {
		for item in stage.items {
			item_records.push((stage.stage_id, item));
		}

		b.push_bind(stage.stage_id)
			.push_bind(trace_id)
			.push_bind(stage.stage_order as i32)
			.push_bind(stage.stage_name)
			.push_bind(stage.stage_payload)
			.push_bind(stage.created_at);
	});
	stage_builder.push(" ON CONFLICT (stage_id) DO NOTHING");
	stage_builder.build().execute(&mut *executor).await?;

	if item_records.is_empty() {
		return Ok(());
	}

	let mut item_builder = QueryBuilder::new(
		"\
INSERT INTO search_trace_stage_items (
	id,
	stage_id,
	item_id,
	note_id,
	chunk_id,
	metrics
) ",
	);

	item_builder.push_values(item_records, |mut b, (stage_id, item)| {
		b.push_bind(item.id)
			.push_bind(stage_id)
			.push_bind(item.item_id)
			.push_bind(item.note_id)
			.push_bind(item.chunk_id)
			.push_bind(item.metrics);
	});
	item_builder.push(" ON CONFLICT (id) DO NOTHING");
	item_builder.build().execute(executor).await?;

	Ok(())
}

async fn persist_trace_inline_header(
	executor: &mut PgConnection,
	trace: &TraceRecord,
) -> Result<()> {
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
	.bind(trace.trace_id)
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
	.bind(trace.config_snapshot.clone())
	.bind(trace.trace_version)
	.bind(trace.created_at)
	.bind(trace.expires_at)
	.execute(executor)
	.await?;

	Ok(())
}

async fn persist_trace_inline_items(
	executor: &mut PgConnection,
	trace_id: Uuid,
	items: Vec<TraceItemRecord>,
) -> Result<()> {
	if items.is_empty() {
		return Ok(());
	}

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
		let explain_json =
			serde_json::to_value(item.explain).expect("SearchExplain must be JSON-serializable.");

		b.push_bind(item.item_id)
			.push_bind(trace_id)
			.push_bind(item.note_id)
			.push_bind(item.chunk_id)
			.push_bind(item.rank as i32)
			.push_bind(item.final_score)
			.push_bind(explain_json);
	});

	builder.push(" ON CONFLICT (item_id) DO NOTHING");
	builder.build().execute(executor).await?;

	Ok(())
}

async fn persist_trace_inline_candidates(
	executor: &mut PgConnection,
	trace_id: Uuid,
	candidates: Vec<TraceCandidateRecord>,
) -> Result<()> {
	if candidates.is_empty() {
		return Ok(());
	}

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
	builder.build().execute(executor).await?;

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

	sqlx::query(
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
	)
	.bind(&hit_ids)
	.bind(&note_ids)
	.bind(&chunk_ids)
	.bind(&ranks)
	.bind(&final_scores)
	.bind(now)
	.bind(query_hash.as_str())
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
	let payload: Option<Value> = sqlx::query_scalar(
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
	)
	.bind(kind.as_str())
	.bind(key)
	.bind(now)
	.fetch_optional(executor)
	.await?;
	let Some(payload) = payload else {
		return Ok(None);
	};
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
	payload: Value,
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

	sqlx::query(
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
	)
	.bind(Uuid::new_v4())
	.bind(kind.as_str())
	.bind(key)
	.bind(payload)
	.bind(now)
	.bind(expires_at)
	.execute(executor)
	.await?;

	Ok(Some(payload_size))
}

#[cfg(test)]
mod tests {
	use crate::search::{
		BlendRankingOverride, ChunkCandidate, ChunkMeta, ChunkSnippet, HashMap, NoteMeta,
		OffsetDateTime, RankingRequestOverride, RerankCacheCandidate, RerankCacheItem,
		RerankCachePayload, RetrievalSourceCandidates, RetrievalSourceKind,
		RetrievalSourcesRankingOverride, ScoredChunk, TraceReplayCandidate, TraceReplayContext,
		Uuid, build_trace_audit, ranking, ranking_policy_id, replay_ranking_from_candidates,
	};
	use elf_config::{Config, SearchDynamic};
	use serde_json::Value;

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

	#[test]
	fn build_trace_audit_includes_token_id_when_present() {
		let audit = build_trace_audit("agent-a", Some("tok-123"));

		assert_eq!(audit.get("actor_id"), Some(&Value::from("agent-a")));
		assert_eq!(audit.get("token_id"), Some(&Value::from("tok-123")));
	}

	#[test]
	fn build_trace_audit_omits_token_id_when_empty() {
		let audit = build_trace_audit("agent-a", Some("   "));

		assert_eq!(audit.get("actor_id"), Some(&Value::from("agent-a")));
		assert!(audit.get("token_id").is_none());
	}

	fn test_chunk_candidate(note_id: Uuid, retrieval_rank: u32) -> ChunkCandidate {
		ChunkCandidate {
			chunk_id: Uuid::new_v4(),
			note_id,
			chunk_index: 0,
			retrieval_rank,
			scope: None,
			updated_at: None,
			embedding_version: Some("v1".to_string()),
		}
	}

	fn default_retrieval_sources_policy() -> ranking::ResolvedRetrievalSourcesPolicy {
		ranking::ResolvedRetrievalSourcesPolicy {
			fusion_weight: 1.0,
			structured_field_weight: 1.0,
			recursive_weight: 0.0,
			fusion_priority: 1,
			structured_field_priority: 0,
			recursive_priority: 0,
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
				scope: None,
				updated_at: None,
				embedding_version: Some("v1".to_string()),
			},
			ChunkCandidate {
				chunk_id: fusion_only_chunk_id,
				note_id: fusion_only_note_id,
				chunk_index: 0,
				retrieval_rank: 1,
				scope: None,
				updated_at: None,
				embedding_version: Some("v1".to_string()),
			},
		];
		let structured = vec![ChunkCandidate {
			chunk_id: shared_chunk_id,
			note_id: shared_note_id,
			chunk_index: 0,
			retrieval_rank: 1,
			scope: None,
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
			agent_id: "agent-a".to_string(),
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
			agent_id: "agent-a".to_string(),
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
			agent_id: "agent-a".to_string(),
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
				recursive_weight: Some(0.0),
				fusion_priority: Some(2),
				structured_field_priority: Some(1),
				recursive_priority: Some(0),
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
