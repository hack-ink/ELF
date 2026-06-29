use std::{net::SocketAddr, sync::Arc};

use axum::{
	Router,
	body::Body,
	extract::State,
	http::{HeaderMap, Request, StatusCode},
	middleware::{self, Next},
	response::IntoResponse,
};
use color_eyre::Result;
use reqwest::{Client, RequestBuilder};
use rmcp::{
	ErrorData, ServerHandler,
	handler::server::router::tool::ToolRouter,
	model::{CallToolResult, JsonObject, ServerCapabilities, ServerInfo},
	transport::streamable_http_server::{
		StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
	},
};
use serde_json::Value;
use tokio::net::TcpListener;
use uuid::Uuid;

use crate::app::McpAuthState;
use elf_config::McpContext;

#[path = "server/runtime.rs"] mod runtime;
#[path = "server/schemas.rs"] mod schemas;
#[path = "server/state.rs"] mod state;
#[path = "server/support.rs"] mod support;

pub use runtime::serve_mcp;
use schemas::*;
use state::{ElfContextHeaders, ElfMcp, HttpMethod};
use support::*;

const HEADER_TENANT_ID: &str = "X-ELF-Tenant-Id";
const HEADER_PROJECT_ID: &str = "X-ELF-Project-Id";
const HEADER_AGENT_ID: &str = "X-ELF-Agent-Id";
const HEADER_READ_PROFILE: &str = "X-ELF-Read-Profile";
const HEADER_REQUEST_ID: &str = "X-ELF-Request-Id";
const HEADER_AUTHORIZATION: &str = "Authorization";

#[rmcp::tool_router]
impl ElfMcp {
	#[rmcp::tool(
		name = "elf_notes_ingest",
		description = "Ingest deterministic notes into ELF. This tool never calls an LLM.",
		input_schema = notes_ingest_schema()
	)]
	async fn elf_notes_ingest(&self, params: JsonObject) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Post, "/v2/notes/ingest", params, None).await
	}

	#[rmcp::tool(
		name = "elf_graph_query",
		description = "Query graph entities and relations by structured criteria.",
		input_schema = graph_query_schema()
	)]
	async fn elf_graph_query(&self, params: JsonObject) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Post, "/v2/graph/query", params, None).await
	}

	#[rmcp::tool(
		name = "elf_graph_report",
		description = "Build a source-backed graph topic map with current, historical, future, inferred, ambiguous, stale, and superseded fact markers.",
		input_schema = graph_report_schema()
	)]
	async fn elf_graph_report(&self, params: JsonObject) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Post, "/v2/graph/report", params, None).await
	}

	#[rmcp::tool(
		name = "elf_events_ingest",
		description = "Ingest an event by extracting evidence-bound notes using the configured LLM extractor.",
		input_schema = events_ingest_schema()
	)]
	async fn elf_events_ingest(&self, params: JsonObject) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Post, "/v2/events/ingest", params, None).await
	}

	#[rmcp::tool(
		name = "elf_docs_put",
		description = "Store a document (evidence source) in ELF Doc Extension v1.",
		input_schema = docs_put_schema()
	)]
	async fn elf_docs_put(&self, params: JsonObject) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Post, "/v2/docs", params, None).await
	}

	#[rmcp::tool(
		name = "elf_docs_get",
		description = "Fetch a single document's metadata by doc_id.",
		input_schema = docs_get_schema()
	)]
	async fn elf_docs_get(&self, mut params: JsonObject) -> Result<CallToolResult, ErrorData> {
		let doc_id = take_required_string(&mut params, "doc_id")?;
		let path = format!("/v2/docs/{doc_id}");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_docs_delete",
		description = "Delete a Source Library document by doc_id and enqueue derived doc-vector removal.",
		input_schema = docs_get_schema()
	)]
	async fn elf_docs_delete(&self, mut params: JsonObject) -> Result<CallToolResult, ErrorData> {
		let doc_id = take_required_string(&mut params, "doc_id")?;
		let path = format!("/v2/docs/{doc_id}");

		self.forward(HttpMethod::Delete, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_docs_search_l0",
		description = "Run a minimal Doc search (L0): chunk-level results with short snippets.",
		input_schema = docs_search_l0_schema()
	)]
	async fn elf_docs_search_l0(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		// read_profile is part of the MCP server configuration and is not client-controlled.
		let _ = take_optional_string(&mut params, "read_profile")?;

		self.forward(HttpMethod::Post, "/v2/docs/search/l0", params, None).await
	}

	#[rmcp::tool(
		name = "elf_docs_excerpts_get",
		description = "Hydrate a verifiable excerpt (L1 or L2) from a stored document.",
		input_schema = docs_excerpts_get_schema()
	)]
	async fn elf_docs_excerpts_get(&self, params: JsonObject) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Post, "/v2/docs/excerpts", params, None).await
	}

	#[rmcp::tool(
		name = "elf_work_journal_entry_create",
		description = "Capture one source-adjacent Work Journal entry with source refs, redaction, next-step, rejected-option, and promotion-boundary metadata. Journal content is not authoritative memory.",
		input_schema = work_journal_entry_create_schema()
	)]
	async fn elf_work_journal_entry_create(
		&self,
		params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Post, "/v2/work-journal/entries", params, None).await
	}

	#[rmcp::tool(
		name = "elf_work_journal_entry_get",
		description = "Fetch one readable Work Journal entry by entry_id.",
		input_schema = work_journal_entry_get_schema()
	)]
	async fn elf_work_journal_entry_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let entry_id = take_required_string(&mut params, "entry_id")?;
		let path = format!("/v2/work-journal/entries/{entry_id}");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_work_journal_session_readback",
		description = "Read newest Work Journal entries for a session and return a where_stopped projection with journal evidence. Current-fact answers still require accepted memory or knowledge authority.",
		input_schema = work_journal_session_readback_schema()
	)]
	async fn elf_work_journal_session_readback(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		// read_profile is part of the MCP server configuration and is not client-controlled.
		let _ = take_optional_string(&mut params, "read_profile")?;

		self.forward(HttpMethod::Post, "/v2/work-journal/readback", params, None).await
	}

	#[rmcp::tool(
		name = "elf_core_blocks_get",
		description = "Fetch core memory blocks explicitly attached to the configured agent and read profile. This is separate from archival search.",
		input_schema = core_blocks_get_schema()
	)]
	async fn elf_core_blocks_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		// read_profile is part of the MCP server configuration and is not client-controlled.
		let _ = take_optional_string(&mut params, "read_profile")?;

		self.forward(HttpMethod::Get, "/v2/core-blocks", params, None).await
	}

	#[rmcp::tool(
		name = "elf_entity_memory_get",
		description = "Fetch an entity-scoped memory view across attached core blocks and graph-linked archival notes.",
		input_schema = entity_memory_get_schema()
	)]
	async fn elf_entity_memory_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		// read_profile is part of the MCP server configuration and is not client-controlled.
		let _ = take_optional_string(&mut params, "read_profile")?;

		self.forward(HttpMethod::Get, "/v2/entity-memory", params, None).await
	}

	#[rmcp::tool(
		name = "elf_dreaming_review_queue",
		description = "List source-backed Dreaming review queue proposals with variants, affected refs, lint flags, policy gates, and review audit.",
		input_schema = dreaming_review_queue_schema()
	)]
	async fn elf_dreaming_review_queue(
		&self,
		params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Get, "/v2/admin/dreaming/review-queue", params, None).await
	}

	#[rmcp::tool(
		name = "elf_recall_debug_panel",
		description = "Build an agent-facing cross-layer recall/debug panel and deterministic recall_trace over memory traces, source documents, knowledge pages, graph facts, and Dreaming proposals.",
		input_schema = recall_debug_panel_schema()
	)]
	async fn elf_recall_debug_panel(
		&self,
		params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		reject_context_override_params(&params)?;

		self.forward(HttpMethod::Post, "/v2/recall-debug/panel", params, None).await
	}

	#[rmcp::tool(
		name = "elf_searches_create",
		description = "Create a search session using quick-find or planned-search mode. Response includes optional trajectory_summary for staged retrieval progress.",
		input_schema = searches_create_schema()
	)]
	async fn elf_searches_create(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		// read_profile is part of the MCP server configuration and is not client-controlled.
		let _ = take_optional_string(&mut params, "read_profile")?;

		self.forward(HttpMethod::Post, "/v2/searches", params, None).await
	}

	#[rmcp::tool(
		name = "elf_searches_get",
		description = "Fetch a search session index view by search_id, including optional trajectory_summary.",
		input_schema = searches_get_schema()
	)]
	async fn elf_searches_get(&self, mut params: JsonObject) -> Result<CallToolResult, ErrorData> {
		let search_id = take_required_string(&mut params, "search_id")?;
		let path = format!("/v2/searches/{search_id}");

		self.forward(HttpMethod::Get, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_searches_timeline",
		description = "Build a timeline view from a search session.",
		input_schema = searches_timeline_schema()
	)]
	async fn elf_searches_timeline(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let search_id = take_required_string(&mut params, "search_id")?;
		let path = format!("/v2/searches/{search_id}/timeline");

		self.forward(HttpMethod::Get, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_searches_notes",
		description = "Fetch note details for selected note_ids from a search session. l0/l1 strip evidence/source_ref; l2 returns full detail.",
		input_schema = searches_notes_schema()
	)]
	async fn elf_searches_notes(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let search_id = take_required_string(&mut params, "search_id")?;
		let path = format!("/v2/searches/{search_id}/notes");

		self.forward(HttpMethod::Post, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_notes_list",
		description = "List notes in a tenant and project with optional filters.",
		input_schema = notes_list_schema()
	)]
	async fn elf_notes_list(&self, params: JsonObject) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Get, "/v2/notes", params, None).await
	}

	#[rmcp::tool(
		name = "elf_notes_get",
		description = "Fetch a single note by note_id.",
		input_schema = notes_get_schema()
	)]
	async fn elf_notes_get(&self, mut params: JsonObject) -> Result<CallToolResult, ErrorData> {
		let note_id = take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/notes/{note_id}");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_notes_patch",
		description = "Patch a note by note_id. Only provided fields are updated.",
		input_schema = notes_patch_schema()
	)]
	async fn elf_notes_patch(&self, mut params: JsonObject) -> Result<CallToolResult, ErrorData> {
		let note_id = take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/notes/{note_id}");

		self.forward(HttpMethod::Patch, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_notes_delete",
		description = "Delete a note by note_id.",
		input_schema = notes_get_schema()
	)]
	async fn elf_notes_delete(&self, mut params: JsonObject) -> Result<CallToolResult, ErrorData> {
		let note_id = take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/notes/{note_id}");

		self.forward(HttpMethod::Delete, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_notes_publish",
		description = "Publish a note from agent_private into a shared space (team_shared or org_shared).",
		input_schema = notes_publish_schema()
	)]
	async fn elf_notes_publish(&self, mut params: JsonObject) -> Result<CallToolResult, ErrorData> {
		let note_id = take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/notes/{note_id}/publish");

		self.forward(HttpMethod::Post, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_notes_unpublish",
		description = "Unpublish a shared note back into agent_private scope.",
		input_schema = notes_unpublish_schema()
	)]
	async fn elf_notes_unpublish(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let note_id = take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/notes/{note_id}/unpublish");

		self.forward(HttpMethod::Post, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_space_grants_list",
		description = "List sharing grants for a space (team_shared or org_shared).",
		input_schema = space_grants_list_schema()
	)]
	async fn elf_space_grants_list(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let space = take_required_string(&mut params, "space")?;
		let path = format!("/v2/spaces/{space}/grants");

		self.forward(HttpMethod::Get, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_space_grant_upsert",
		description = "Upsert a sharing grant for a space (team_shared or org_shared).",
		input_schema = space_grant_upsert_schema()
	)]
	async fn elf_space_grant_upsert(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let space = take_required_string(&mut params, "space")?;
		let path = format!("/v2/spaces/{space}/grants");

		self.forward(HttpMethod::Post, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_space_grant_revoke",
		description = "Revoke a sharing grant for a space (team_shared or org_shared).",
		input_schema = space_grant_revoke_schema()
	)]
	async fn elf_space_grant_revoke(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let space = take_required_string(&mut params, "space")?;
		let path = format!("/v2/spaces/{space}/grants/revoke");

		self.forward(HttpMethod::Post, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_admin_traces_recent_list",
		description = "List recent traces by tenant/project with optional cursor and filters.",
		input_schema = admin_traces_recent_list_schema()
	)]
	async fn elf_admin_traces_recent_list(
		&self,
		params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Get, "/v2/admin/traces/recent", params, None).await
	}

	#[rmcp::tool(
		name = "elf_admin_trace_get",
		description = "Fetch trace metadata, items, and optional trajectory summary by trace_id.",
		input_schema = admin_trace_get_schema()
	)]
	async fn elf_admin_trace_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let trace_id = take_required_string(&mut params, "trace_id")?;
		let path = format!("/v2/admin/traces/{trace_id}");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_admin_trajectory_get",
		description = "Fetch trace trajectory and stage payload by trace_id.",
		input_schema = admin_trajectory_get_schema()
	)]
	async fn elf_admin_trajectory_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let trace_id = take_required_string(&mut params, "trace_id")?;
		let path = format!("/v2/admin/trajectories/{trace_id}");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_admin_trace_item_get",
		description = "Fetch a trace item explain payload by item_id.",
		input_schema = admin_trace_item_get_schema()
	)]
	async fn elf_admin_trace_item_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let item_id = take_required_string(&mut params, "item_id")?;
		let path = format!("/v2/admin/trace-items/{item_id}");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_admin_note_provenance_get",
		description = "Fetch provenance bundle and related history for one note.",
		input_schema = admin_note_provenance_get_schema()
	)]
	async fn elf_admin_note_provenance_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let note_id = take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/admin/notes/{note_id}/provenance");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_admin_memory_history_get",
		description = "Fetch chronological memory history for one note.",
		input_schema = admin_memory_history_get_schema()
	)]
	async fn elf_admin_memory_history_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let note_id = take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/admin/notes/{note_id}/history");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_admin_trace_bundle_get",
		description = "Fetch trace bundle for replay and diagnostics by trace_id.",
		input_schema = admin_trace_bundle_get_schema()
	)]
	async fn elf_admin_trace_bundle_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let trace_id = take_required_string(&mut params, "trace_id")?;
		let path = format!("/v2/admin/traces/{trace_id}/bundle");

		self.forward(HttpMethod::Get, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_admin_events_ingestion_profiles_list",
		description = "List latest ingestion profiles for add_event.",
		input_schema = admin_ingestion_profiles_list_schema()
	)]
	async fn elf_admin_events_ingestion_profiles_list(
		&self,
		_params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		self.forward(
			HttpMethod::Get,
			"/v2/admin/events/ingestion-profiles",
			JsonObject::new(),
			None,
		)
		.await
	}

	#[rmcp::tool(
		name = "elf_admin_events_ingestion_profiles_create",
		description = "Create a new ingestion profile version for add_event.",
		input_schema = admin_ingestion_profiles_create_schema()
	)]
	async fn elf_admin_events_ingestion_profiles_create(
		&self,
		params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Post, "/v2/admin/events/ingestion-profiles", params, None).await
	}

	#[rmcp::tool(
		name = "elf_admin_events_ingestion_profile_get",
		description = "Get a single ingestion profile by id/version for add_event.",
		input_schema = admin_ingestion_profile_get_schema()
	)]
	async fn elf_admin_events_ingestion_profile_get(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let profile_id = take_required_string(&mut params, "profile_id")?;
		let path = format!("/v2/admin/events/ingestion-profiles/{profile_id}");

		self.forward(HttpMethod::Get, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_admin_events_ingestion_profile_versions_list",
		description = "List all versions of one ingestion profile for add_event.",
		input_schema = admin_ingestion_profile_versions_list_schema()
	)]
	async fn elf_admin_events_ingestion_profile_versions_list(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		let profile_id = take_required_string(&mut params, "profile_id")?;
		let path = format!("/v2/admin/events/ingestion-profiles/{profile_id}/versions");

		self.forward(HttpMethod::Get, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_admin_events_ingestion_profile_default_get",
		description = "Get the active default ingestion profile for add_event.",
		input_schema = admin_ingestion_profile_default_get_schema()
	)]
	async fn elf_admin_events_ingestion_profile_default_get(
		&self,
		_params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		self.forward(
			HttpMethod::Get,
			"/v2/admin/events/ingestion-profiles/default",
			JsonObject::new(),
			None,
		)
		.await
	}

	#[rmcp::tool(
		name = "elf_admin_events_ingestion_profile_default_set",
		description = "Set the default ingestion profile for add_event.",
		input_schema = admin_ingestion_profile_default_set_schema()
	)]
	async fn elf_admin_events_ingestion_profile_default_set(
		&self,
		params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Put, "/v2/admin/events/ingestion-profiles/default", params, None)
			.await
	}
}

#[cfg(test)]
#[path = "server/tests.rs"]
mod tests;
