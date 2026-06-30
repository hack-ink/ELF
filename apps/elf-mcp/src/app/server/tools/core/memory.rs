use color_eyre::Result;
use rmcp::{
	ErrorData,
	model::{CallToolResult, JsonObject},
};

use crate::app::server::{
	ElfMcp, HttpMethod,
	schemas::{
		core_blocks_get_schema, dreaming_review_queue_schema, entity_memory_get_schema,
		recall_debug_panel_schema, work_journal_entry_create_schema, work_journal_entry_get_schema,
		work_journal_session_readback_schema,
	},
	support,
};

#[rmcp::tool_router(router = core_memory_tool_router, vis = "pub(in crate::app::server)")]
impl ElfMcp {
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
}
