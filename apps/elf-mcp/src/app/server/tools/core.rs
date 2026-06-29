use color_eyre::Result;
use rmcp::{
	ErrorData,
	model::{CallToolResult, JsonObject},
};

use crate::app::server::{
	ElfMcp, HttpMethod,
	schemas::{
		core_blocks_get_schema, dreaming_review_queue_schema, entity_memory_get_schema,
		events_ingest_schema, graph_query_schema, graph_report_schema, notes_get_schema,
		notes_ingest_schema, notes_list_schema, notes_patch_schema, notes_publish_schema,
		notes_unpublish_schema, recall_debug_panel_schema, searches_create_schema,
		searches_get_schema, searches_notes_schema, searches_timeline_schema,
		space_grant_revoke_schema, space_grant_upsert_schema, space_grants_list_schema,
		work_journal_entry_create_schema, work_journal_entry_get_schema,
		work_journal_session_readback_schema,
	},
	support,
};

#[rmcp::tool_router(router = core_tool_router, vis = "pub(in crate::app::server)")]
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
		let entry_id = support::take_required_string(&mut params, "entry_id")?;
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
		let _ = support::take_optional_string(&mut params, "read_profile")?;

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
		let _ = support::take_optional_string(&mut params, "read_profile")?;

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
		let _ = support::take_optional_string(&mut params, "read_profile")?;

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
	pub(in crate::app::server) async fn elf_recall_debug_panel(
		&self,
		params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		support::reject_context_override_params(&params)?;

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
		let _ = support::take_optional_string(&mut params, "read_profile")?;

		self.forward(HttpMethod::Post, "/v2/searches", params, None).await
	}

	#[rmcp::tool(
		name = "elf_searches_get",
		description = "Fetch a search session index view by search_id, including optional trajectory_summary.",
		input_schema = searches_get_schema()
	)]
	async fn elf_searches_get(&self, mut params: JsonObject) -> Result<CallToolResult, ErrorData> {
		let search_id = support::take_required_string(&mut params, "search_id")?;
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
		let search_id = support::take_required_string(&mut params, "search_id")?;
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
		let search_id = support::take_required_string(&mut params, "search_id")?;
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
		let note_id = support::take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/notes/{note_id}");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_notes_patch",
		description = "Patch a note by note_id. Only provided fields are updated.",
		input_schema = notes_patch_schema()
	)]
	async fn elf_notes_patch(&self, mut params: JsonObject) -> Result<CallToolResult, ErrorData> {
		let note_id = support::take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/notes/{note_id}");

		self.forward(HttpMethod::Patch, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_notes_delete",
		description = "Delete a note by note_id.",
		input_schema = notes_get_schema()
	)]
	async fn elf_notes_delete(&self, mut params: JsonObject) -> Result<CallToolResult, ErrorData> {
		let note_id = support::take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/notes/{note_id}");

		self.forward(HttpMethod::Delete, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_notes_publish",
		description = "Publish a note from agent_private into a shared space (team_shared or org_shared).",
		input_schema = notes_publish_schema()
	)]
	async fn elf_notes_publish(&self, mut params: JsonObject) -> Result<CallToolResult, ErrorData> {
		let note_id = support::take_required_string(&mut params, "note_id")?;
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
		let note_id = support::take_required_string(&mut params, "note_id")?;
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
		let space = support::take_required_string(&mut params, "space")?;
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
		let space = support::take_required_string(&mut params, "space")?;
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
		let space = support::take_required_string(&mut params, "space")?;
		let path = format!("/v2/spaces/{space}/grants/revoke");

		self.forward(HttpMethod::Post, &path, params, None).await
	}
}
