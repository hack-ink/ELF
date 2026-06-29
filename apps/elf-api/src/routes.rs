//! HTTP route builders and request handlers.

use axum::{
	Json, Router,
	body::{self, Body},
	extract::{
		DefaultBodyLimit, Extension, Path, Query, State,
		rejection::{JsonRejection, QueryRejection},
	},
	http::{
		HeaderMap, HeaderValue, Request, StatusCode,
		header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE},
	},
	middleware::{self, Next},
	response::{IntoResponse, Response},
	routing,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};
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

/// JSON OpenAPI contract route.
pub const OPENAPI_JSON_PATH: &str = "/openapi.json";
/// Scalar API reference route.
pub const SCALAR_DOCS_PATH: &str = "/docs";
/// Local read-only admin viewer route.
pub const ADMIN_VIEWER_PATH: &str = "/viewer";

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
const VIEWER_HTML: &str = include_str!("../static/viewer.html");

#[path = "routes/admin_notes.rs"] mod admin_notes;
#[path = "routes/admin_ops.rs"] mod admin_ops;
#[path = "routes/consolidation.rs"] mod consolidation;
#[path = "routes/core_memory.rs"] mod core_memory;
#[path = "routes/docs.rs"] mod docs;
#[path = "routes/dreaming.rs"] mod dreaming;
#[path = "routes/events.rs"] mod events;
#[path = "routes/graph.rs"] mod graph;
#[path = "routes/health.rs"] mod health;
#[path = "routes/ingestion_profiles.rs"] mod ingestion_profiles;
#[path = "routes/knowledge.rs"] mod knowledge;
#[path = "routes/notes.rs"] mod notes;
#[path = "routes/recall.rs"] mod recall;
#[path = "routes/search.rs"] mod search;
#[path = "routes/sharing.rs"] mod sharing;
#[path = "routes/support.rs"] mod support;
#[path = "routes/trace.rs"] mod trace;
#[path = "routes/types.rs"] mod types;
#[path = "routes/work_journal.rs"] mod work_journal;

use admin_notes::*;
use admin_ops::*;
use consolidation::*;
use core_memory::*;
use docs::*;
use dreaming::*;
use events::*;
use graph::*;
use health::*;
use ingestion_profiles::*;
use knowledge::*;
use notes::*;
use recall::*;
use search::*;
use sharing::*;
use support::*;
use trace::*;
use types::*;
use work_journal::*;

/// Generated OpenAPI document for the ELF HTTP API.
#[derive(OpenApi)]
#[openapi(
	info(
		title = "ELF API",
		version = env!("CARGO_PKG_VERSION"),
		description = "Evidence-linked fact memory HTTP and admin API."
	),
	paths(
		health,
		notes_ingest,
		events_ingest,
		docs_put,
		docs_get,
		docs_delete,
		docs_search_l0,
		docs_excerpts_get,
		core_blocks_get,
		entity_memory_get,
		admin_core_block_upsert,
		admin_core_block_attach,
		admin_core_block_detach,
		admin_docs_get,
		admin_docs_search_l0,
		admin_docs_excerpts_get,
		graph_query,
		searches_create,
		searches_get,
		searches_timeline,
		searches_notes,
		notes_list,
		notes_get,
		notes_patch,
		notes_delete,
		notes_publish,
		notes_unpublish,
		work_journal_entry_create,
		work_journal_entry_get,
		work_journal_session_readback,
		space_grants_list,
		space_grant_upsert,
		space_grant_revoke,
		admin_ingestion_profiles_list,
		admin_ingestion_profile_create,
		admin_ingestion_profile_get,
		admin_ingestion_profile_versions_list,
		admin_ingestion_profile_default_get,
		admin_ingestion_profile_default_set,
		consolidation_run_create,
		consolidation_runs_list,
		consolidation_run_get,
		consolidation_proposals_list,
		consolidation_proposal_get,
		consolidation_proposal_review,
		dreaming_review_queue,
		recall_debug_panel,
		knowledge_page_rebuild,
		knowledge_pages_watch_rebuild,
		knowledge_pages_list,
		knowledge_pages_search,
		knowledge_page_get,
		knowledge_page_lint,
		rebuild_qdrant,
		searches_raw,
		trace_recent_list,
		trace_get,
		trace_bundle_get,
		trace_trajectory_get,
		trace_item_get,
		admin_graph_predicates_list,
		admin_graph_predicate_patch,
			admin_graph_predicate_alias_add,
			admin_graph_predicate_aliases_list,
			admin_note_provenance_get,
			admin_note_history_get,
			admin_note_correction_apply,
		),
	components(schemas(
		AdminIngestionProfileDefaultResponseV2,
		AdminIngestionProfileDefaultSetBody,
		ErrorBody,
	)),
	tags(
		(name = "health", description = "Health and process liveness."),
		(name = "notes", description = "Memory note ingestion, listing, mutation, and sharing."),
		(name = "events", description = "Event ingestion through the extractor pipeline."),
		(name = "docs", description = "Document extension ingestion, search, and excerpt retrieval."),
		(name = "search", description = "Progressive search sessions and raw search diagnostics."),
		(name = "graph", description = "Graph query and predicate administration."),
		(name = "consolidation", description = "Reviewable derived consolidation proposals."),
		(name = "dreaming", description = "Dreaming review queue and derived memory organization."),
		(name = "recall", description = "Cross-layer recall and debug readback."),
		(name = "knowledge", description = "Derived knowledge page rebuild and lint readback."),
		(name = "work_journal", description = "Source-adjacent Work Journal capture and session readback."),
		(name = "admin", description = "Local admin and operator inspection routes."),
	)
)]
pub struct ApiDoc;

/// Builds the authenticated public API router.
pub fn router(state: AppState) -> Router {
	let auth_state = state.clone();
	let api_router = Router::new()
		.route("/health", routing::get(health))
		.route("/v2/notes/ingest", routing::post(notes_ingest))
		.route("/v2/events/ingest", routing::post(events_ingest))
		.route("/v2/core-blocks", routing::get(core_blocks_get))
		.route("/v2/entity-memory", routing::get(entity_memory_get))
		.route("/v2/recall-debug/panel", routing::post(recall_debug_panel))
		.route("/v2/searches", routing::post(searches_create))
		.route("/v2/searches/{search_id}", routing::get(searches_get))
		.route("/v2/searches/{search_id}/timeline", routing::get(searches_timeline))
		.route("/v2/searches/{search_id}/notes", routing::post(searches_notes))
		.route("/v2/graph/query", routing::post(graph_query))
		.route("/v2/graph/report", routing::post(graph_report))
		.route("/v2/notes", routing::get(notes_list))
		.route(
			"/v2/notes/{note_id}",
			routing::get(notes_get).patch(notes_patch).delete(notes_delete),
		)
		.route("/v2/notes/{note_id}/publish", routing::post(notes_publish))
		.route("/v2/notes/{note_id}/unpublish", routing::post(notes_unpublish))
		.route("/v2/work-journal/entries", routing::post(work_journal_entry_create))
		.route("/v2/work-journal/entries/{entry_id}", routing::get(work_journal_entry_get))
		.route("/v2/work-journal/readback", routing::post(work_journal_session_readback))
		.route(
			"/v2/spaces/{space}/grants",
			routing::get(space_grants_list).post(space_grant_upsert),
		)
		.route("/v2/spaces/{space}/grants/revoke", routing::post(space_grant_revoke))
		.with_state(state.clone())
		.layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES));
	let docs_router = Router::new()
		.route("/v2/docs", routing::post(docs_put))
		.route("/v2/docs/{doc_id}", routing::get(docs_get).delete(docs_delete))
		.route("/v2/docs/search/l0", routing::post(docs_search_l0))
		.route("/v2/docs/excerpts", routing::post(docs_excerpts_get))
		.with_state(state)
		.layer(DefaultBodyLimit::max(MAX_DOC_REQUEST_BYTES));

	Router::new()
		.merge(contract_router())
		.merge(api_router)
		.merge(docs_router)
		.layer(middleware::from_fn_with_state(auth_state, api_auth_middleware))
}

/// Builds the authenticated admin API router.
pub fn admin_router(state: AppState) -> Router {
	let auth_state = state.clone();
	let protected_router = Router::new()
		.route("/v2/admin/searches", routing::post(searches_create))
		.route("/v2/admin/searches/{search_id}", routing::get(searches_get))
		.route("/v2/admin/searches/{search_id}/timeline", routing::get(searches_timeline))
		.route("/v2/admin/searches/{search_id}/notes", routing::post(searches_notes))
		.route("/v2/admin/core-blocks", routing::post(admin_core_block_upsert))
		.route(
			"/v2/admin/core-blocks/{block_id}/attachments",
			routing::post(admin_core_block_attach),
		)
		.route(
			"/v2/admin/core-blocks/attachments/{attachment_id}",
			routing::delete(admin_core_block_detach),
		)
		.route("/v2/admin/docs/search/l0", routing::post(admin_docs_search_l0))
		.route("/v2/admin/docs/excerpts", routing::post(admin_docs_excerpts_get))
		.route("/v2/admin/docs/{doc_id}", routing::get(admin_docs_get))
		.route("/v2/admin/notes", routing::get(notes_list))
		.route("/v2/admin/notes/{note_id}", routing::get(notes_get))
		.route(
			"/v2/admin/events/ingestion-profiles/default",
			routing::get(admin_ingestion_profile_default_get)
				.put(admin_ingestion_profile_default_set),
		)
		.route(
			"/v2/admin/events/ingestion-profiles/{profile_id}/versions",
			routing::get(admin_ingestion_profile_versions_list),
		)
		.route(
			"/v2/admin/events/ingestion-profiles/{profile_id}",
			routing::get(admin_ingestion_profile_get),
		)
		.route(
			"/v2/admin/events/ingestion-profiles",
			routing::get(admin_ingestion_profiles_list).post(admin_ingestion_profile_create),
		)
		.route(
			"/v2/admin/consolidation/runs",
			routing::get(consolidation_runs_list).post(consolidation_run_create),
		)
		.route("/v2/admin/consolidation/runs/{run_id}", routing::get(consolidation_run_get))
		.route("/v2/admin/consolidation/proposals", routing::get(consolidation_proposals_list))
		.route(
			"/v2/admin/consolidation/proposals/{proposal_id}",
			routing::get(consolidation_proposal_get),
		)
		.route(
			"/v2/admin/consolidation/proposals/{proposal_id}/review",
			routing::post(consolidation_proposal_review),
		)
		.route("/v2/admin/dreaming/review-queue", routing::get(dreaming_review_queue))
		.route("/v2/admin/recall-debug/panel", routing::post(admin_recall_debug_panel))
		.route("/v2/admin/knowledge/pages", routing::get(knowledge_pages_list))
		.route("/v2/admin/knowledge/pages/rebuild", routing::post(knowledge_page_rebuild))
		.route(
			"/v2/admin/knowledge/pages/rebuild-changed-sources",
			routing::post(knowledge_pages_watch_rebuild),
		)
		.route("/v2/admin/knowledge/pages/search", routing::post(knowledge_pages_search))
		.route("/v2/admin/knowledge/pages/{page_id}", routing::get(knowledge_page_get))
		.route("/v2/admin/knowledge/pages/{page_id}/lint", routing::post(knowledge_page_lint))
		.route("/v2/admin/qdrant/rebuild", routing::post(rebuild_qdrant))
		.route("/v2/admin/searches/raw", routing::post(searches_raw))
		.route("/v2/admin/traces/recent", routing::get(trace_recent_list))
		.route("/v2/admin/traces/{trace_id}", routing::get(trace_get))
		.route("/v2/admin/traces/{trace_id}/bundle", routing::get(trace_bundle_get))
		.route("/v2/admin/trajectories/{trace_id}", routing::get(trace_trajectory_get))
		.route("/v2/admin/trace-items/{item_id}", routing::get(trace_item_get))
		.route("/v2/admin/graph/predicates", routing::get(admin_graph_predicates_list))
		.route(
			"/v2/admin/graph/predicates/{predicate_id}",
			routing::patch(admin_graph_predicate_patch),
		)
		.route(
			"/v2/admin/graph/predicates/{predicate_id}/aliases",
			routing::post(admin_graph_predicate_alias_add).get(admin_graph_predicate_aliases_list),
		)
		.route("/v2/admin/notes/{note_id}/provenance", routing::get(admin_note_provenance_get))
		.route("/v2/admin/notes/{note_id}/history", routing::get(admin_note_history_get))
		.route("/v2/admin/notes/{note_id}/corrections", routing::post(admin_note_correction_apply))
		.with_state(state)
		.layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
		.layer(middleware::from_fn_with_state(auth_state, admin_auth_middleware));

	Router::new()
		.route(ADMIN_VIEWER_PATH, routing::get(admin_viewer))
		.route("/", routing::get(admin_viewer))
		.merge(protected_router)
}

/// Builds the API contract router.
pub fn contract_router<S>() -> Router<S>
where
	S: Clone + Send + Sync + 'static,
{
	Router::new()
		.route(OPENAPI_JSON_PATH, routing::get(openapi_json))
		.merge(Scalar::with_url(SCALAR_DOCS_PATH, <ApiDoc as OpenApi>::openapi()))
}

async fn openapi_json() -> Response {
	let mut response = Json(<ApiDoc as OpenApi>::openapi()).into_response();

	response
		.headers_mut()
		.insert(CONTENT_TYPE, HeaderValue::from_static("application/vnd.oai.openapi+json"));

	response
}

async fn admin_viewer() -> Response {
	let mut response = VIEWER_HTML.into_response();

	response
		.headers_mut()
		.insert(CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"));
	response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));

	response
}

#[cfg(test)]
#[path = "routes/tests.rs"]
mod tests;
