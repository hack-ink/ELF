use color_eyre::Result;
use rmcp::{
	ErrorData,
	model::{CallToolResult, JsonObject},
};

use crate::app::server::{
	ElfMcp, HttpMethod,
	schemas::{events_ingest_schema, graph_query_schema, graph_report_schema, notes_ingest_schema},
};

#[rmcp::tool_router(router = core_ingest_tool_router, vis = "pub(in crate::app::server)")]
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
}
