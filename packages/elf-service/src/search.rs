//! Search APIs and ranking explanations.

mod api;
mod cache;
mod db_helpers;
mod filter;
mod finish;
mod helpers;
mod hits;
mod item_builders;
mod query_plan;
mod ranking;
mod replay_helpers;
mod retrieval;
mod scoring_helpers;
mod service;
mod sql;
mod state;
mod structured;
mod trace;
mod trace_persistence;
mod trace_stages;
mod trajectory_loaders;

pub use crate::ranking_explain_v2::{SearchRankingExplain, SearchRankingTerm};
pub use api::{
	BlendRankingOverride, BlendSegmentOverride, DiversityRankingOverride, PayloadLevel, QueryPlan,
	QueryPlanBlendSegment, QueryPlanBudget, QueryPlanDynamicGate, QueryPlanFusionPolicy,
	QueryPlanIntent, QueryPlanRerankPolicy, QueryPlanRetrievalStage, QueryPlanRewrite,
	QueryPlanStage, RankingRequestOverride, RecentTraceHeader, RetrievalSourcesRankingOverride,
	SearchDiversityExplain, SearchExplain, SearchExplainItem, SearchExplainRelationContext,
	SearchExplainRelationContextObject, SearchExplainRelationEntityRef, SearchExplainRequest,
	SearchExplainResponse, SearchExplainTrajectory, SearchExplainTrajectoryMatch,
	SearchExplainTrajectoryStage, SearchItem, SearchMatchExplain, SearchRawPlannedResponse,
	SearchRequest, SearchResponse, SearchTrace, SearchTrajectoryResponse, SearchTrajectoryStage,
	SearchTrajectoryStageItem, SearchTrajectorySummary, SearchTrajectorySummaryStage,
	TraceBundleGetRequest, TraceBundleMode, TraceBundleResponse, TraceGetRequest, TraceGetResponse,
	TraceRecentCursor, TraceRecentListRequest, TraceRecentListResponse, TraceReplayCandidate,
	TraceReplayContext, TraceReplayItem, TraceTrajectoryGetRequest,
};

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
use sqlx::{FromRow, PgConnection, PgExecutor, PgPool, QueryBuilder};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
	ElfService, Result,
	access::ORG_PROJECT_ID,
	graph::RelationTemporalStatus,
	ranking_explain_v2::{SEARCH_RANKING_EXPLAIN_SCHEMA_V2, TraceTermsArgs},
};
use cache::{fetch_cache_payload, store_cache_payload};
use db_helpers::{fetch_chunks_by_pair, fetch_note_vectors_for_diversity};
use elf_config::{Config, SearchCache};
use elf_domain::english_gate;
use elf_storage::{
	models::MemoryNote,
	qdrant::{BM25_MODEL, BM25_VECTOR_NAME, DENSE_VECTOR_NAME},
};
use filter::{SearchFilter, SearchFilterImpact};
use helpers::{
	apply_payload_level_to_search_item, build_search_filter, build_trajectory_summary_from_stages,
	raw_search_path_label, sorted_unique_strings, validate_search_request_inputs,
};
use hits::record_hits;
use item_builders::{build_search_item_and_trace_item, build_trace_candidate_record};
use ranking::{
	NormalizationKind, ResolvedBlendPolicy, ResolvedDiversityPolicy, ResolvedRetrievalSourcesPolicy,
};
use replay_helpers::cmp_scored_replay;
use scoring_helpers::{score_chunk_candidate, select_best_scored_chunks};
use sql::{
	DEFAULT_BOUNDED_CANDIDATES_LIMIT, DEFAULT_BOUNDED_STAGE_ITEMS_LIMIT,
	DEFAULT_FULL_CANDIDATES_LIMIT, DEFAULT_FULL_STAGE_ITEMS_LIMIT, DEFAULT_RECENT_TRACES_LIMIT,
	MAX_RECENT_TRACES_LIMIT, MAX_TRACE_BUNDLE_CANDIDATES_LIMIT, MAX_TRACE_BUNDLE_ITEMS_LIMIT,
	RECENT_TRACES_SCHEMA_V1, RELATION_CONTEXT_SQL, SEARCH_FILTER_IMPACT_SCHEMA_V1,
	SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1, TRACE_BUNDLE_SCHEMA_V1,
};
use state::{
	BestChunkForNoteRow, BuildQueryPlanArgs, BuildSearchItemArgs, BuildTraceArgs, CacheKind,
	CachePayload, ChunkCandidate, ChunkMeta, ChunkRow, ChunkSnippet, DeterministicRankingTerms,
	DiversityDecision, DynamicGateSummary, ExpansionCachePayload, ExpansionMode, ExpansionOutput,
	FieldHit, FinishSearchArgs, FinishSearchPolicies, FinishSearchScoringResult,
	MaybeDynamicSearchArgs, NoteMeta, NoteVectorRow, QueryEmbedding, QueryPlanStagesArgs,
	RawSearchExecutionContext, RawSearchPath, RecursiveRetrievalArgs, RecursiveRetrievalResult,
	RerankCacheCandidate, RerankCacheItem, RerankCachePayload, RetrievalSourceCandidates,
	RetrievalSourceKind, ScoreCandidateCtx, ScoreSnippetArgs, ScoredChunk, ScoredReplay,
	SearchExplainTraceRow, SearchRecentTraceRow, SearchRelationContextRow, SearchRetrievalArgs,
	SearchRetrievalResult, SearchTraceBuilder, SearchTraceItemRow, SearchTraceRow,
	StructuredFieldHitArgs, StructuredFieldHitRow, StructuredFieldRetrievalArgs,
	StructuredFieldRetrievalResult, TraceCandidateRecord, TraceCandidateSnapshotRow, TraceContext,
	TraceItemRecord, TracePayload, TraceRecord, TraceTrajectoryStageItemRecord,
	TraceTrajectoryStageRecord,
};
use structured::{build_structured_field_candidates, build_structured_field_matches};
use trace_persistence::{enqueue_trace, persist_trace_inline};
use trace_stages::{build_trace_audit, build_trace_trajectory_stages};
use trajectory_loaders::{
	load_item_trajectory, load_trace_trajectory_stages, load_trace_trajectory_summary,
};

const TRACE_VERSION: i32 = 3;
const MAX_MATCHED_TERMS: usize = 8;
const MAX_TRAJECTORY_STAGE_ITEMS: usize = 256;
const MAX_CANDIDATE_K: u32 = 1_024;
pub(crate) fn resolve_read_profile_scopes(cfg: &Config, profile: &str) -> Result<Vec<String>> {
	ranking::resolve_scopes(cfg, profile)
}

/// Computes the stable ranking-policy identifier for a search configuration.
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

/// Replays ranking against stored trace candidates and returns the final top-k items.
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
	let det_query_tokens = structured::build_deterministic_query_tokens(cfg, trace.query.as_str());
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
		let scored = replay_helpers::score_replay_candidate(&score_ctx, candidate, rerank_rank);
		let replace = match best_by_note.get(&candidate.note_id) {
			None => true,
			Some(existing) => replay_helpers::should_replace_replay_best(existing, &scored),
		};

		if replace {
			best_by_note.insert(candidate.note_id, scored);
		}
	}

	let mut results: Vec<ScoredReplay> = best_by_note.into_values().collect();

	results.sort_by(cmp_scored_replay);

	let results = replay_helpers::apply_replay_diversity_selection(
		results,
		top_k,
		diversity_policy.enabled,
		&replay_diversity_decisions,
	);

	Ok(replay_helpers::build_replay_items(
		cfg,
		&blend_policy,
		&diversity_policy,
		policy_id.as_str(),
		&replay_diversity_decisions,
		results,
	))
}

#[cfg(test)]
#[path = "search/tests.rs"]
mod tests;
