use color_eyre::Result;
use rmcp::{
	ErrorData,
	model::{CallToolResult, JsonObject},
};

use crate::app::server::{
	ElfMcp, HttpMethod,
	schemas::{
		notes_get_schema, notes_list_schema, notes_patch_schema, notes_publish_schema,
		notes_unpublish_schema,
	},
	support,
};

#[rmcp::tool_router(router = core_notes_tool_router, vis = "pub(in crate::app::server)")]
impl ElfMcp {
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
}
