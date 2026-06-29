#![cfg_attr(test, allow(unused_crate_dependencies))]

//! Service-layer request models and orchestration for ELF.

pub mod add_event;
pub mod add_note;
pub mod admin;
pub mod admin_graph_predicates;
pub mod consolidation;
pub mod core_blocks;
pub mod delete;
pub mod docs;
pub mod dreaming_review_queue;
pub mod entity_memory;
pub mod graph;
pub mod graph_query;
pub mod graph_report;
pub mod knowledge;
pub mod list;
pub mod memory_corrections;
pub mod notes;
pub mod progressive_search;
pub mod provenance;
pub mod recall_debug;
pub mod search;
pub mod sharing;
pub mod structured_fields;
pub mod time_serde;
pub mod update;
pub mod work_journal;

mod access;
mod constants;
mod error;
mod graph_ingestion;
mod history;
mod ingest_audit;
mod ingestion_profiles;
mod ops;
mod providers;
mod ranking_explain_v2;
mod service;
mod update_resolution;
mod vectors;
mod write_policy;

pub use self::{
	add_event::{AddEventRequest, AddEventResponse, AddEventResult, EventMessage},
	add_note::{AddNoteInput, AddNoteRequest, AddNoteResponse, AddNoteResult},
	admin::RebuildReport,
	admin_graph_predicates::{
		AdminGraphPredicateAliasAddRequest, AdminGraphPredicateAliasResponse,
		AdminGraphPredicateAliasesListRequest, AdminGraphPredicateAliasesResponse,
		AdminGraphPredicatePatchRequest, AdminGraphPredicateResponse,
		AdminGraphPredicatesListRequest, AdminGraphPredicatesListResponse,
	},
	consolidation::{
		ConsolidationProposalGetRequest, ConsolidationProposalInput, ConsolidationProposalResponse,
		ConsolidationProposalReviewEventResponse, ConsolidationProposalReviewRequest,
		ConsolidationProposalsListRequest, ConsolidationProposalsListResponse,
		ConsolidationRunCreateRequest, ConsolidationRunCreateResponse, ConsolidationRunGetRequest,
		ConsolidationRunResponse, ConsolidationRunsListRequest, ConsolidationRunsListResponse,
	},
	constants::{REJECT_EVIDENCE_MISMATCH, REJECT_WRITE_POLICY_MISMATCH},
	core_blocks::{
		CoreBlockAttachRequest, CoreBlockAttachResponse, CoreBlockDetachRequest,
		CoreBlockDetachResponse, CoreBlockItem, CoreBlockRecord, CoreBlockUpsertRequest,
		CoreBlockUpsertResponse, CoreBlocksGetRequest, CoreBlocksResponse,
		ELF_CORE_MEMORY_BLOCKS_SCHEMA_V1,
	},
	delete::{DeleteRequest, DeleteResponse},
	docs::{
		DocType, DocsDeleteRequest, DocsDeleteResponse, DocsExcerptResponse,
		DocsExcerptsGetRequest, DocsGetRequest, DocsGetResponse, DocsPutRequest, DocsPutResponse,
		DocsSearchL0Request, DocsSearchL0Response, TextPositionSelector, TextQuoteSelector,
	},
	dreaming_review_queue::{
		DreamingReviewQueueAudit, DreamingReviewQueueItem, DreamingReviewQueueItemPolicy,
		DreamingReviewQueuePolicy, DreamingReviewQueueRequest, DreamingReviewQueueResponse,
		DreamingReviewQueueSummary, ELF_DREAMING_REVIEW_QUEUE_SCHEMA_V1,
	},
	entity_memory::{
		ELF_ENTITY_MEMORY_VIEW_SCHEMA_V1, EntityMemoryEntity, EntityMemoryItem,
		EntityMemoryRelation, EntityMemorySummary, EntityMemoryViewRequest,
		EntityMemoryViewResponse,
	},
	error::{Error, Result},
	graph::RelationTemporalStatus,
	graph_query::{
		ELF_GRAPH_QUERY_SCHEMA_V1, GraphQueryEntity, GraphQueryEntityRef, GraphQueryExplain,
		GraphQueryFact, GraphQueryObject, GraphQueryObjectEntity, GraphQueryPredicate,
		GraphQueryPredicateRef, GraphQueryRequest, GraphQueryResponse,
	},
	graph_report::{
		ELF_GRAPH_REPORT_SCHEMA_V1, GraphReportEntity, GraphReportExplain, GraphReportFact,
		GraphReportPredicate, GraphReportRequest, GraphReportResponse, GraphReportSummary,
		GraphTopicEdge, GraphTopicMap, GraphTopicNode,
	},
	ingestion_profiles::{
		AdminIngestionProfileCreateRequest, AdminIngestionProfileDefaultGetRequest,
		AdminIngestionProfileDefaultResponse, AdminIngestionProfileDefaultSetRequest,
		AdminIngestionProfileGetRequest, AdminIngestionProfileListRequest,
		AdminIngestionProfileResponse, AdminIngestionProfileSummary,
		AdminIngestionProfileVersionsListRequest, AdminIngestionProfileVersionsListResponse,
		AdminIngestionProfilesListResponse, IngestionProfileRef, IngestionProfileSelector,
	},
	knowledge::{
		KnowledgeDeltaMemoryCandidate, KnowledgePageChangedSource, KnowledgePageGetRequest,
		KnowledgePageLintFindingResponse, KnowledgePageLintRequest, KnowledgePageLintResponse,
		KnowledgePageLintSummary, KnowledgePageProposalRunSummary, KnowledgePageRebuildOutput,
		KnowledgePageRebuildRequest, KnowledgePageRebuildResponse, KnowledgePageResponse,
		KnowledgePageSearchItem, KnowledgePageSearchRequest, KnowledgePageSearchResponse,
		KnowledgePageSectionRebuildState, KnowledgePageSectionResponse,
		KnowledgePageSectionSourceBacklink, KnowledgePageSourceRefResponse, KnowledgePageSummary,
		KnowledgePageWatchRebuildRequest, KnowledgePageWatchRebuildResponse,
		KnowledgePageWatchRebuildSummary, KnowledgePagesListRequest, KnowledgePagesListResponse,
	},
	list::{ListItem, ListRequest, ListResponse},
	memory_corrections::{
		MemoryCorrectionAction, MemoryCorrectionRequest, MemoryCorrectionResponse,
	},
	notes::{NoteFetchRequest, NoteFetchResponse},
	ops::NoteOp,
	progressive_search::{
		SearchDetailsError, SearchDetailsRequest, SearchDetailsResponse, SearchDetailsResult,
		SearchIndexItem, SearchIndexPlannedResponse, SearchIndexResponse, SearchSessionGetRequest,
		SearchTimelineGroup, SearchTimelineRequest, SearchTimelineResponse,
	},
	provenance::{
		MemoryHistoryEvent, MemoryHistoryGetRequest, MemoryHistoryResponse,
		NoteProvenanceBundleResponse, NoteProvenanceGetRequest, NoteProvenanceIndexingOutbox,
		NoteProvenanceIngestDecision, NoteProvenanceNote, NoteProvenanceNoteVersion,
		NoteProvenanceRecentTrace,
	},
	providers::{BoxFuture, EmbeddingProvider, ExtractorProvider, Providers, RerankProvider},
	recall_debug::{
		ELF_RECALL_DEBUG_PANEL_SCHEMA_V1, ELF_RECALL_TRACE_SCHEMA_V1, RecallDebugLayer,
		RecallDebugPanelRequest, RecallDebugPanelRequestEcho, RecallDebugPanelResponse,
		RecallDebugPanelSummary, RecallDebugRow, RecallTrace, RecallTraceEntry, RecallTraceSummary,
	},
	search::{
		BlendRankingOverride, BlendSegmentOverride, PayloadLevel, QueryPlan, QueryPlanBlendSegment,
		QueryPlanBudget, QueryPlanDynamicGate, QueryPlanFusionPolicy, QueryPlanIntent,
		QueryPlanRerankPolicy, QueryPlanRetrievalStage, QueryPlanRewrite, QueryPlanStage,
		RankingRequestOverride, SearchExplain, SearchExplainItem, SearchExplainRequest,
		SearchExplainResponse, SearchExplainTrajectory, SearchExplainTrajectoryStage, SearchItem,
		SearchRawPlannedResponse, SearchRequest, SearchResponse, SearchTrace,
		SearchTrajectoryResponse, SearchTrajectoryStage, SearchTrajectoryStageItem,
		SearchTrajectorySummary, SearchTrajectorySummaryStage, TraceBundleGetRequest,
		TraceBundleResponse, TraceGetRequest, TraceGetResponse, TraceRecentListRequest,
		TraceRecentListResponse, TraceTrajectoryGetRequest,
	},
	service::ElfService,
	sharing::{
		GranteeKind, PublishNoteRequest, PublishNoteResponse, ShareScope, SpaceGrantItem,
		SpaceGrantRevokeRequest, SpaceGrantRevokeResponse, SpaceGrantUpsertRequest,
		SpaceGrantUpsertResponse, SpaceGrantsListRequest, SpaceGrantsListResponse,
		UnpublishNoteRequest, UnpublishNoteResponse,
	},
	structured_fields::StructuredFields,
	update::{UpdateRequest, UpdateResponse},
	work_journal::{
		ELF_WORK_JOURNAL_SCHEMA_V1, WorkJournalEntryCreateRequest, WorkJournalEntryCreateResponse,
		WorkJournalEntryFamily, WorkJournalEntryGetRequest, WorkJournalEntryResponse,
		WorkJournalSessionReadbackRequest, WorkJournalSessionReadbackResponse,
		WorkJournalWhereStopped,
	},
};

pub(crate) use self::{
	history::{InsertVersionArgs, enqueue_outbox_tx, insert_version, note_snapshot},
	update_resolution::{
		ResolveUpdateArgs, UpdateDecision, UpdateDecisionMetadata, resolve_update,
	},
	vectors::{embedding_version, parse_pg_vector, vector_to_pg},
	write_policy::writegate_reason_code,
};
