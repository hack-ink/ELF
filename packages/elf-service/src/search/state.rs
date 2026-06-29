mod cache;
mod finish;
mod modes;
mod records;
mod retrieval;
mod scoring;
mod trace;

pub(super) use self::{
	cache::{
		CacheKind, CachePayload, ExpansionCachePayload, ExpansionOutput, RerankCacheItem,
		RerankCachePayload,
	},
	finish::{
		BuildQueryPlanArgs, BuildSearchItemArgs, BuildTraceArgs, FinishSearchArgs,
		FinishSearchPolicies, FinishSearchScoringResult, QueryPlanStagesArgs,
		RawSearchExecutionContext,
	},
	modes::{ExpansionMode, RawSearchPath, RetrievalSourceKind},
	records::{
		BestChunkForNoteRow, ChunkMeta, ChunkRow, ChunkSnippet, NoteMeta, NoteVectorRow,
		SearchExplainTraceRow, SearchRecentTraceRow, SearchRelationContextRow, SearchTraceItemRow,
		SearchTraceRow, StructuredFieldHitRow, TraceCandidateSnapshotRow,
	},
	retrieval::{
		ChunkCandidate, DynamicGateSummary, FieldHit, MaybeDynamicSearchArgs, QueryEmbedding,
		RecursiveRetrievalArgs, RecursiveRetrievalResult, RerankCacheCandidate,
		RetrievalSourceCandidates, SearchRetrievalArgs, SearchRetrievalResult,
		StructuredFieldHitArgs, StructuredFieldRetrievalArgs, StructuredFieldRetrievalResult,
	},
	scoring::{
		DeterministicRankingTerms, DiversityDecision, ScoreCandidateCtx, ScoreSnippetArgs,
		ScoredChunk, ScoredReplay,
	},
	trace::{
		SearchTraceBuilder, TraceCandidateRecord, TraceContext, TraceItemRecord, TracePayload,
		TraceRecord, TraceTrajectoryStageItemRecord, TraceTrajectoryStageRecord,
	},
};
