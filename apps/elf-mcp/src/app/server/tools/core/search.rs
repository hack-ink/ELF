use color_eyre::Result;
use rmcp::{
	ErrorData,
	model::{CallToolResult, JsonObject},
};

use crate::app::server::{
	ElfMcp, HttpMethod,
	schemas::{
		searches_create_schema, searches_get_schema, searches_notes_schema,
		searches_timeline_schema,
	},
	support,
};

#[rmcp::tool_router(router = core_search_tool_router, vis = "pub(in crate::app::server)")]
impl ElfMcp {
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
}
