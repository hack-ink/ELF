use color_eyre::Result;
use rmcp::{
	ErrorData,
	model::{CallToolResult, JsonObject},
};

use crate::app::server::{
	ElfMcp, HttpMethod,
	schemas::{docs_excerpts_get_schema, docs_get_schema, docs_put_schema, docs_search_l0_schema},
	support,
};

#[rmcp::tool_router(router = docs_tool_router, vis = "pub(in crate::app::server)")]
impl ElfMcp {
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
		let doc_id = support::take_required_string(&mut params, "doc_id")?;
		let path = format!("/v2/docs/{doc_id}");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_docs_delete",
		description = "Delete a Source Library document by doc_id and enqueue derived doc-vector removal.",
		input_schema = docs_get_schema()
	)]
	async fn elf_docs_delete(&self, mut params: JsonObject) -> Result<CallToolResult, ErrorData> {
		let doc_id = support::take_required_string(&mut params, "doc_id")?;
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
		let _ = support::take_optional_string(&mut params, "read_profile")?;

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
}
