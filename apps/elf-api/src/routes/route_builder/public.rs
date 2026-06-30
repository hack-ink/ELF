use axum::{Router, routing};

use crate::{routes, state::AppState};

pub(super) fn public_api_router() -> Router<AppState> {
	Router::new()
		.route("/health", routing::get(routes::health::health))
		.route("/v2/notes/ingest", routing::post(routes::notes::notes_ingest))
		.route("/v2/events/ingest", routing::post(routes::events::events_ingest))
		.route("/v2/core-blocks", routing::get(routes::core_memory::core_blocks_get))
		.route("/v2/entity-memory", routing::get(routes::core_memory::entity_memory_get))
		.route("/v2/recall-debug/panel", routing::post(routes::recall::recall_debug_panel))
		.route("/v2/searches", routing::post(routes::search::searches_create))
		.route("/v2/searches/{search_id}", routing::get(routes::search::searches_get))
		.route("/v2/searches/{search_id}/timeline", routing::get(routes::search::searches_timeline))
		.route("/v2/searches/{search_id}/notes", routing::post(routes::search::searches_notes))
		.route("/v2/graph/query", routing::post(routes::graph::graph_query))
		.route("/v2/graph/report", routing::post(routes::graph::graph_report))
		.route("/v2/notes", routing::get(routes::notes::notes_list))
		.route(
			"/v2/notes/{note_id}",
			routing::get(routes::notes::notes_get)
				.patch(routes::notes::notes_patch)
				.delete(routes::notes::notes_delete),
		)
		.route("/v2/notes/{note_id}/publish", routing::post(routes::notes::notes_publish))
		.route("/v2/notes/{note_id}/unpublish", routing::post(routes::notes::notes_unpublish))
		.route(
			"/v2/work-journal/entries",
			routing::post(routes::work_journal::work_journal_entry_create),
		)
		.route(
			"/v2/work-journal/entries/{entry_id}",
			routing::get(routes::work_journal::work_journal_entry_get),
		)
		.route(
			"/v2/work-journal/readback",
			routing::post(routes::work_journal::work_journal_session_readback),
		)
		.route(
			"/v2/spaces/{space}/grants",
			routing::get(routes::sharing::space_grants_list)
				.post(routes::sharing::space_grant_upsert),
		)
		.route(
			"/v2/spaces/{space}/grants/revoke",
			routing::post(routes::sharing::space_grant_revoke),
		)
}

pub(super) fn docs_api_router() -> Router<AppState> {
	Router::new()
		.route("/v2/docs", routing::post(routes::docs::docs_put))
		.route(
			"/v2/docs/{doc_id}",
			routing::get(routes::docs::docs_get).delete(routes::docs::docs_delete),
		)
		.route("/v2/docs/search/l0", routing::post(routes::docs::docs_search_l0))
		.route("/v2/docs/excerpts", routing::post(routes::docs::docs_excerpts_get))
}
