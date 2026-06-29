use axum::{
	Json, Router,
	http::{HeaderValue, header::CONTENT_TYPE},
	response::{IntoResponse, Response},
	routing,
};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::routes::{
	admin_notes::{
		__path_admin_note_correction_apply, __path_admin_note_history_get,
		__path_admin_note_provenance_get,
	},
	admin_ops::__path_rebuild_qdrant,
	consolidation::{
		__path_consolidation_proposal_get, __path_consolidation_proposal_review,
		__path_consolidation_proposals_list, __path_consolidation_run_create,
		__path_consolidation_run_get, __path_consolidation_runs_list,
	},
	core_memory::{
		__path_admin_core_block_attach, __path_admin_core_block_detach,
		__path_admin_core_block_upsert, __path_core_blocks_get, __path_entity_memory_get,
	},
	docs::{
		__path_admin_docs_excerpts_get, __path_admin_docs_get, __path_admin_docs_search_l0,
		__path_docs_delete, __path_docs_excerpts_get, __path_docs_get, __path_docs_put,
		__path_docs_search_l0,
	},
	dreaming::__path_dreaming_review_queue,
	events::__path_events_ingest,
	graph::{
		__path_admin_graph_predicate_alias_add, __path_admin_graph_predicate_aliases_list,
		__path_admin_graph_predicate_patch, __path_admin_graph_predicates_list, __path_graph_query,
		__path_graph_report,
	},
	health::__path_health,
	ingestion_profiles::{
		__path_admin_ingestion_profile_create, __path_admin_ingestion_profile_default_get,
		__path_admin_ingestion_profile_default_set, __path_admin_ingestion_profile_get,
		__path_admin_ingestion_profile_versions_list, __path_admin_ingestion_profiles_list,
	},
	knowledge::{
		__path_knowledge_page_get, __path_knowledge_page_lint, __path_knowledge_page_rebuild,
		__path_knowledge_pages_list, __path_knowledge_pages_search,
		__path_knowledge_pages_watch_rebuild,
	},
	notes::{
		__path_notes_delete, __path_notes_get, __path_notes_ingest, __path_notes_list,
		__path_notes_patch, __path_notes_publish, __path_notes_unpublish,
	},
	recall::__path_recall_debug_panel,
	search::{
		__path_searches_create, __path_searches_get, __path_searches_notes, __path_searches_raw,
		__path_searches_timeline,
	},
	sharing::{__path_space_grant_revoke, __path_space_grant_upsert, __path_space_grants_list},
	trace::{
		__path_trace_bundle_get, __path_trace_get, __path_trace_item_get, __path_trace_recent_list,
		__path_trace_trajectory_get,
	},
	types::{
		AdminIngestionProfileDefaultResponseV2, AdminIngestionProfileDefaultSetBody, ErrorBody,
	},
	work_journal::{
		__path_work_journal_entry_create, __path_work_journal_entry_get,
		__path_work_journal_session_readback,
	},
};

/// JSON OpenAPI contract route.
pub const OPENAPI_JSON_PATH: &str = "/openapi.json";
/// Scalar API reference route.
pub const SCALAR_DOCS_PATH: &str = "/docs";

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
		graph_report,
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
