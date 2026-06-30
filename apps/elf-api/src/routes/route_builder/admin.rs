use axum::{Router, extract::DefaultBodyLimit, middleware, routing};

use crate::{
	routes::{self, ADMIN_VIEWER_PATH, MAX_REQUEST_BYTES},
	state::AppState,
};

pub(super) fn admin_router(state: AppState) -> Router {
	let auth_state = state.clone();
	let protected_router = Router::new()
		.merge(admin_search_routes())
		.merge(admin_core_routes())
		.merge(admin_docs_routes())
		.merge(admin_notes_routes())
		.merge(admin_ingestion_profile_routes())
		.merge(admin_consolidation_routes())
		.merge(admin_knowledge_routes())
		.merge(admin_trace_routes())
		.merge(admin_graph_routes())
		.merge(admin_ops_routes())
		.with_state(state)
		.layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
		.layer(middleware::from_fn_with_state(auth_state, routes::support::admin_auth_middleware));

	Router::new()
		.route(ADMIN_VIEWER_PATH, routing::get(routes::viewer::admin_viewer))
		.route("/", routing::get(routes::viewer::admin_viewer))
		.merge(protected_router)
}

fn admin_search_routes() -> Router<AppState> {
	Router::new()
		.route("/v2/admin/searches", routing::post(routes::search::searches_create))
		.route("/v2/admin/searches/raw", routing::post(routes::search::searches_raw))
		.route("/v2/admin/searches/{search_id}", routing::get(routes::search::searches_get))
		.route(
			"/v2/admin/searches/{search_id}/timeline",
			routing::get(routes::search::searches_timeline),
		)
		.route(
			"/v2/admin/searches/{search_id}/notes",
			routing::post(routes::search::searches_notes),
		)
}

fn admin_core_routes() -> Router<AppState> {
	Router::new()
		.route("/v2/admin/core-blocks", routing::post(routes::core_memory::admin_core_block_upsert))
		.route(
			"/v2/admin/core-blocks/{block_id}/attachments",
			routing::post(routes::core_memory::admin_core_block_attach),
		)
		.route(
			"/v2/admin/core-blocks/attachments/{attachment_id}",
			routing::delete(routes::core_memory::admin_core_block_detach),
		)
		.route(
			"/v2/admin/recall-debug/panel",
			routing::post(routes::recall::admin_recall_debug_panel),
		)
}

fn admin_docs_routes() -> Router<AppState> {
	Router::new()
		.route("/v2/admin/docs/search/l0", routing::post(routes::docs::admin_docs_search_l0))
		.route("/v2/admin/docs/excerpts", routing::post(routes::docs::admin_docs_excerpts_get))
		.route("/v2/admin/docs/{doc_id}", routing::get(routes::docs::admin_docs_get))
}

fn admin_notes_routes() -> Router<AppState> {
	Router::new()
		.route("/v2/admin/notes", routing::get(routes::notes::notes_list))
		.route("/v2/admin/notes/{note_id}", routing::get(routes::notes::notes_get))
		.route(
			"/v2/admin/notes/{note_id}/provenance",
			routing::get(routes::admin_notes::admin_note_provenance_get),
		)
		.route(
			"/v2/admin/notes/{note_id}/history",
			routing::get(routes::admin_notes::admin_note_history_get),
		)
		.route(
			"/v2/admin/notes/{note_id}/corrections",
			routing::post(routes::admin_notes::admin_note_correction_apply),
		)
}

fn admin_ingestion_profile_routes() -> Router<AppState> {
	Router::new()
		.route(
			"/v2/admin/events/ingestion-profiles/default",
			routing::get(routes::ingestion_profiles::admin_ingestion_profile_default_get)
				.put(routes::ingestion_profiles::admin_ingestion_profile_default_set),
		)
		.route(
			"/v2/admin/events/ingestion-profiles/{profile_id}/versions",
			routing::get(routes::ingestion_profiles::admin_ingestion_profile_versions_list),
		)
		.route(
			"/v2/admin/events/ingestion-profiles/{profile_id}",
			routing::get(routes::ingestion_profiles::admin_ingestion_profile_get),
		)
		.route(
			"/v2/admin/events/ingestion-profiles",
			routing::get(routes::ingestion_profiles::admin_ingestion_profiles_list)
				.post(routes::ingestion_profiles::admin_ingestion_profile_create),
		)
}

fn admin_consolidation_routes() -> Router<AppState> {
	Router::new()
		.route(
			"/v2/admin/consolidation/runs",
			routing::get(routes::consolidation::consolidation_runs_list)
				.post(routes::consolidation::consolidation_run_create),
		)
		.route(
			"/v2/admin/consolidation/runs/{run_id}",
			routing::get(routes::consolidation::consolidation_run_get),
		)
		.route(
			"/v2/admin/consolidation/proposals",
			routing::get(routes::consolidation::consolidation_proposals_list),
		)
		.route(
			"/v2/admin/consolidation/proposals/{proposal_id}",
			routing::get(routes::consolidation::consolidation_proposal_get),
		)
		.route(
			"/v2/admin/consolidation/proposals/{proposal_id}/review",
			routing::post(routes::consolidation::consolidation_proposal_review),
		)
		.route(
			"/v2/admin/dreaming/review-queue",
			routing::get(routes::dreaming::dreaming_review_queue),
		)
}

fn admin_knowledge_routes() -> Router<AppState> {
	Router::new()
		.route("/v2/admin/knowledge/pages", routing::get(routes::knowledge::knowledge_pages_list))
		.route(
			"/v2/admin/knowledge/pages/rebuild",
			routing::post(routes::knowledge::knowledge_page_rebuild),
		)
		.route(
			"/v2/admin/knowledge/pages/rebuild-changed-sources",
			routing::post(routes::knowledge::knowledge_pages_watch_rebuild),
		)
		.route(
			"/v2/admin/knowledge/pages/search",
			routing::post(routes::knowledge::knowledge_pages_search),
		)
		.route(
			"/v2/admin/knowledge/pages/{page_id}",
			routing::get(routes::knowledge::knowledge_page_get),
		)
		.route(
			"/v2/admin/knowledge/pages/{page_id}/lint",
			routing::post(routes::knowledge::knowledge_page_lint),
		)
}

fn admin_trace_routes() -> Router<AppState> {
	Router::new()
		.route("/v2/admin/traces/recent", routing::get(routes::trace::trace_recent_list))
		.route("/v2/admin/traces/{trace_id}", routing::get(routes::trace::trace_get))
		.route("/v2/admin/traces/{trace_id}/bundle", routing::get(routes::trace::trace_bundle_get))
		.route(
			"/v2/admin/trajectories/{trace_id}",
			routing::get(routes::trace::trace_trajectory_get),
		)
		.route("/v2/admin/trace-items/{item_id}", routing::get(routes::trace::trace_item_get))
}

fn admin_graph_routes() -> Router<AppState> {
	Router::new()
		.route(
			"/v2/admin/graph/predicates",
			routing::get(routes::graph::admin_graph_predicates_list),
		)
		.route(
			"/v2/admin/graph/predicates/{predicate_id}",
			routing::patch(routes::graph::admin_graph_predicate_patch),
		)
		.route(
			"/v2/admin/graph/predicates/{predicate_id}/aliases",
			routing::post(routes::graph::admin_graph_predicate_alias_add)
				.get(routes::graph::admin_graph_predicate_aliases_list),
		)
}

fn admin_ops_routes() -> Router<AppState> {
	Router::new()
		.route("/v2/admin/qdrant/rebuild", routing::post(routes::admin_ops::rebuild_qdrant))
}
