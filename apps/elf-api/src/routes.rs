//! HTTP route builders and request handlers.

mod admin_notes;
mod admin_ops;
mod consolidation;
mod contract;
mod core_memory;
mod docs;
mod dreaming;
mod events;
mod graph;
mod health;
mod ingestion_profiles;
mod knowledge;
mod notes;
mod recall;
mod route_builder;
mod search;
mod sharing;
mod support;
mod trace;
mod types;
mod viewer;
mod work_journal;

pub use self::{
	contract::{ApiDoc, OPENAPI_JSON_PATH, SCALAR_DOCS_PATH, contract_router},
	route_builder::{admin_router, router},
	viewer::ADMIN_VIEWER_PATH,
};

use axum::{
	Json,
	body::{self, Body},
	extract::{
		Extension, Path, Query, State,
		rejection::{JsonRejection, QueryRejection},
	},
	http::{
		HeaderMap, Request, StatusCode,
		header::{CONTENT_LENGTH, CONTENT_TYPE},
	},
	middleware::Next,
	response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::state::AppState;
use elf_config::{SecurityAuthKey, SecurityAuthRole};
use elf_domain::{
	consolidation::{
		ConsolidationInputRef, ConsolidationLineage, ConsolidationReviewAction,
		ConsolidationReviewState,
	},
	english_gate,
	knowledge::{KnowledgePageKind, KnowledgeSourceKind},
	writegate::WritePolicy,
};
use elf_service::{
	AddEventRequest, AddEventResponse, AddNoteInput, AddNoteRequest, AddNoteResponse,
	AdminGraphPredicateAliasAddRequest, AdminGraphPredicateAliasesListRequest,
	AdminGraphPredicateAliasesResponse, AdminGraphPredicatePatchRequest,
	AdminGraphPredicateResponse, AdminGraphPredicatesListRequest, AdminGraphPredicatesListResponse,
	AdminIngestionProfileCreateRequest, AdminIngestionProfileDefaultGetRequest,
	AdminIngestionProfileDefaultResponse, AdminIngestionProfileDefaultSetRequest,
	AdminIngestionProfileGetRequest, AdminIngestionProfileListRequest,
	AdminIngestionProfileResponse, AdminIngestionProfileVersionsListRequest,
	AdminIngestionProfileVersionsListResponse, AdminIngestionProfilesListResponse,
	ConsolidationProposalGetRequest, ConsolidationProposalInput, ConsolidationProposalResponse,
	ConsolidationProposalReviewRequest, ConsolidationProposalsListRequest,
	ConsolidationProposalsListResponse, ConsolidationRunCreateRequest,
	ConsolidationRunCreateResponse, ConsolidationRunGetRequest, ConsolidationRunResponse,
	ConsolidationRunsListRequest, ConsolidationRunsListResponse, CoreBlockAttachRequest,
	CoreBlockAttachResponse, CoreBlockDetachRequest, CoreBlockDetachResponse,
	CoreBlockUpsertRequest, CoreBlockUpsertResponse, CoreBlocksGetRequest, CoreBlocksResponse,
	DeleteRequest, DeleteResponse, DocType, DocsDeleteRequest, DocsDeleteResponse,
	DocsExcerptResponse, DocsExcerptsGetRequest, DocsGetRequest, DocsGetResponse, DocsPutRequest,
	DocsPutResponse, DocsSearchL0Request, DocsSearchL0Response, DreamingReviewQueueRequest,
	DreamingReviewQueueResponse, EntityMemoryViewRequest, EntityMemoryViewResponse, Error,
	EventMessage, GranteeKind, GraphQueryEntityRef, GraphQueryPredicateRef, GraphQueryRequest,
	GraphQueryResponse, GraphReportRequest, GraphReportResponse, IngestionProfileSelector,
	KnowledgePageChangedSource, KnowledgePageGetRequest, KnowledgePageLintRequest,
	KnowledgePageLintResponse, KnowledgePageRebuildRequest, KnowledgePageRebuildResponse,
	KnowledgePageResponse, KnowledgePageSearchRequest, KnowledgePageSearchResponse,
	KnowledgePageWatchRebuildRequest, KnowledgePageWatchRebuildResponse, KnowledgePagesListRequest,
	KnowledgePagesListResponse, ListRequest, ListResponse, MemoryCorrectionAction,
	MemoryCorrectionRequest, MemoryCorrectionResponse, MemoryHistoryGetRequest,
	MemoryHistoryResponse, NoteFetchRequest, NoteFetchResponse, NoteProvenanceBundleResponse,
	NoteProvenanceGetRequest, PayloadLevel, PublishNoteRequest, QueryPlan, RankingRequestOverride,
	RebuildReport, RecallDebugPanelRequest, RecallDebugPanelResponse, SearchDetailsRequest,
	SearchDetailsResult, SearchExplainRequest, SearchExplainResponse, SearchIndexItem,
	SearchRequest, SearchResponse, SearchSessionGetRequest, SearchTimelineGroup,
	SearchTimelineRequest, SearchTrajectoryResponse, SearchTrajectorySummary, ShareScope,
	SpaceGrantRevokeRequest, SpaceGrantRevokeResponse, SpaceGrantUpsertRequest,
	SpaceGrantsListRequest, TextPositionSelector, TextQuoteSelector, TraceBundleGetRequest,
	TraceBundleResponse, TraceGetRequest, TraceGetResponse, TraceRecentListRequest,
	TraceRecentListResponse, TraceTrajectoryGetRequest, UnpublishNoteRequest, UpdateRequest,
	UpdateResponse, WorkJournalEntryCreateRequest, WorkJournalEntryCreateResponse,
	WorkJournalEntryFamily, WorkJournalEntryGetRequest, WorkJournalEntryResponse,
	WorkJournalSessionReadbackRequest, WorkJournalSessionReadbackResponse, search::TraceBundleMode,
};
use support::{
	ApiError, EntityMemoryQuery, RequestContext, SearchMode, effective_token_id, empty_json_object,
	format_scope, format_space, json_error, parse_optional_rfc3339, parse_space,
	require_admin_for_org_shared_writes, required_read_profile,
};
#[cfg(test)]
use support::{
	apply_auth_key_context, inject_request_id_into_json_body, parse_request_id_from_headers,
	resolve_auth_key, sanitize_trusted_token_header,
};
use types::{
	AdminGraphPredicateAliasAddBody, AdminGraphPredicatePatchBody, AdminGraphPredicatesListQuery,
	AdminIngestionProfileCreateBody, AdminIngestionProfileDefaultResponseV2,
	AdminIngestionProfileDefaultSetBody, AdminIngestionProfileGetQuery, AdminNoteCorrectionBody,
	ConsolidationProposalReviewBody, ConsolidationProposalsListQuery, ConsolidationRunCreateBody,
	ConsolidationRunsListQuery, CoreBlockAttachBody, CoreBlockUpsertBody, DocsExcerptsGetBody,
	DocsPutBody, DocsSearchL0Body, DreamingReviewQueueQuery, ErrorBody, EventsIngestRequest,
	GraphQueryBody, GraphReportBody, KnowledgePageRebuildBody, KnowledgePageWatchRebuildBody,
	KnowledgePagesListQuery, KnowledgePagesSearchBody, NotePatchRequest, NotesIngestRequest,
	NotesListQuery, PublishResponseV2, RecallDebugPanelBody, SearchCreateRequest,
	SearchCreateResponseV2, SearchDetailsBody, SearchDetailsResponseV2, SearchIndexResponseV2,
	SearchSessionGetQuery, SearchTimelineQuery, SearchTimelineResponseV2, ShareScopeBody,
	SpaceGrantItemV2, SpaceGrantUpsertBody, SpaceGrantUpsertResponseV2, SpaceGrantsListResponseV2,
	TraceBundleGetQuery, TraceRecentListQuery, WorkJournalEntryCreateBody,
	WorkJournalSessionReadbackBody,
};
#[cfg(test)] use viewer::VIEWER_HTML;

const HEADER_TENANT_ID: &str = "X-ELF-Tenant-Id";
const HEADER_PROJECT_ID: &str = "X-ELF-Project-Id";
const HEADER_AGENT_ID: &str = "X-ELF-Agent-Id";
const HEADER_REQUEST_ID: &str = "X-ELF-Request-Id";
const HEADER_READ_PROFILE: &str = "X-ELF-Read-Profile";
const HEADER_AUTHORIZATION: &str = "Authorization";
const HEADER_TRUSTED_TOKEN_ID: &str = "X-ELF-Trusted-Token-Id";
const MAX_CONTEXT_HEADER_CHARS: usize = 128;
const MAX_REQUEST_BYTES: usize = 1_048_576;
const MAX_DOC_REQUEST_BYTES: usize = 4 * 1_024 * 1_024;
const MAX_NOTES_PER_INGEST: usize = 256;
const MAX_MESSAGES_PER_EVENT: usize = 256;
const MAX_MESSAGE_CHARS: usize = 16_384;
const MAX_QUERY_CHARS: usize = 2_048;
const DOC_STATUSES: [&str; 2] = ["active", "deleted"];
const MAX_NOTE_IDS_PER_DETAILS: usize = 256;
const MAX_TOP_K: u32 = 100;
const MAX_CANDIDATE_K: u32 = 1_000;
const MAX_ERROR_LOG_CHARS: usize = 1_024;

#[cfg(test)] mod tests;
