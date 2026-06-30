use color_eyre::Result;
use rmcp::{
	ErrorData,
	model::{CallToolResult, JsonObject},
};

use crate::app::server::{
	ElfMcp, HttpMethod,
	schemas::{space_grant_revoke_schema, space_grant_upsert_schema, space_grants_list_schema},
	support,
};

#[rmcp::tool_router(router = core_sharing_tool_router, vis = "pub(in crate::app::server)")]
impl ElfMcp {
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
