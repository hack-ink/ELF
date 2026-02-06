use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use color_eyre::Result;
use reqwest::Client;
use rmcp::{
	ErrorData as McpError, ServerHandler,
	handler::server::router::tool::ToolRouter,
	model::{CallToolResult, JsonObject, ServerCapabilities, ServerInfo},
	transport::streamable_http_server::{
		StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
	},
};
use serde_json::Value;
use tokio::net::TcpListener;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HttpMethod {
	Get,
	Post,
}

#[derive(Clone)]
struct ElfMcp {
	api_base: String,
	client: Client,
	tool_router: ToolRouter<Self>,
}

impl ElfMcp {
	fn new(api_base: String) -> Self {
		Self { api_base, client: Client::new(), tool_router: Self::tool_router() }
	}

	async fn forward_post(&self, path: &str, body: Value) -> Result<CallToolResult, McpError> {
		let url = format!("{}{}", self.api_base, path);
		let response = self.client.post(url).json(&body).send().await.map_err(|err| {
			McpError::internal_error(format!("ELF API request failed: {err}"), None)
		})?;
		handle_response(response).await
	}

	async fn forward_get(
		&self,
		path: &str,
		params: JsonObject,
	) -> Result<CallToolResult, McpError> {
		let url = format!("{}{}", self.api_base, path);
		let query = params_to_query(params);
		let response = self.client.get(url).query(&query).send().await.map_err(|err| {
			McpError::internal_error(format!("ELF API request failed: {err}"), None)
		})?;
		handle_response(response).await
	}

	async fn forward(
		&self,
		method: HttpMethod,
		path: &str,
		params: JsonObject,
	) -> Result<CallToolResult, McpError> {
		match method {
			HttpMethod::Post => self.forward_post(path, Value::Object(params)).await,
			HttpMethod::Get => self.forward_get(path, params).await,
		}
	}
}

#[rmcp::tool_router]
impl ElfMcp {
	#[rmcp::tool(
        name = "memory_add_note",
        description = "Add memory notes.",
        input_schema = any_json_schema()
    )]
	async fn memory_add_note(&self, params: JsonObject) -> Result<CallToolResult, McpError> {
		self.forward(HttpMethod::Post, "/v1/memory/add_note", params).await
	}

	#[rmcp::tool(
        name = "memory_add_event",
        description = "Add memory extracted from event messages.",
        input_schema = any_json_schema()
    )]
	async fn memory_add_event(&self, params: JsonObject) -> Result<CallToolResult, McpError> {
		self.forward(HttpMethod::Post, "/v1/memory/add_event", params).await
	}

	#[rmcp::tool(
        name = "memory_search",
        description = "Search memory notes.",
        input_schema = any_json_schema()
    )]
	async fn memory_search(&self, params: JsonObject) -> Result<CallToolResult, McpError> {
		self.forward(HttpMethod::Post, "/v1/memory/search", params).await
	}

	#[rmcp::tool(
        name = "memory_list",
        description = "List memory notes.",
        input_schema = any_json_schema()
    )]
	async fn memory_list(&self, params: JsonObject) -> Result<CallToolResult, McpError> {
		self.forward(HttpMethod::Get, "/v1/memory/list", params).await
	}

	#[rmcp::tool(
        name = "memory_search_timeline",
        description = "Build a timeline view from a search session.",
        input_schema = any_json_schema()
    )]
	async fn memory_search_timeline(&self, params: JsonObject) -> Result<CallToolResult, McpError> {
		self.forward(HttpMethod::Post, "/v1/memory/search/timeline", params).await
	}

	#[rmcp::tool(
        name = "memory_search_details",
        description = "Fetch full note details for selected ids from a search session.",
        input_schema = any_json_schema()
    )]
	async fn memory_search_details(&self, params: JsonObject) -> Result<CallToolResult, McpError> {
		self.forward(HttpMethod::Post, "/v1/memory/search/details", params).await
	}

	#[rmcp::tool(
        name = "memory_update",
        description = "Update memory notes.",
        input_schema = any_json_schema()
    )]
	async fn memory_update(&self, params: JsonObject) -> Result<CallToolResult, McpError> {
		self.forward(HttpMethod::Post, "/v1/memory/update", params).await
	}

	#[rmcp::tool(
        name = "memory_delete",
        description = "Delete memory notes.",
        input_schema = any_json_schema()
    )]
	async fn memory_delete(&self, params: JsonObject) -> Result<CallToolResult, McpError> {
		self.forward(HttpMethod::Post, "/v1/memory/delete", params).await
	}
}

#[rmcp::tool_handler]
impl ServerHandler for ElfMcp {
	fn get_info(&self) -> ServerInfo {
		ServerInfo {
			instructions: Some(
				"ELF MCP adapter that forwards tool calls to the ELF HTTP API.".to_string(),
			),
			capabilities: ServerCapabilities::builder().enable_tools().build(),
			..Default::default()
		}
	}
}

pub async fn serve_mcp(bind_addr: &str, api_base: &str) -> Result<()> {
	let bind_addr: SocketAddr = bind_addr.parse()?;
	let api_base = normalize_api_base(api_base);
	let session_manager: Arc<LocalSessionManager> = Default::default();
	let service = StreamableHttpService::new(
		move || Ok(ElfMcp::new(api_base.clone())),
		session_manager,
		StreamableHttpServerConfig::default(),
	);
	let router = Router::new().fallback_service(service);
	let listener = TcpListener::bind(bind_addr).await?;
	axum::serve(listener, router).await?;
	Ok(())
}

fn normalize_api_base(raw: &str) -> String {
	let trimmed = raw.trim_end_matches('/');
	if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
		trimmed.to_string()
	} else {
		format!("http://{trimmed}")
	}
}

fn params_to_query(params: JsonObject) -> Vec<(String, String)> {
	params
		.into_iter()
		.filter_map(|(key, value)| match value {
			Value::Null => None,
			Value::String(text) => Some((key, text)),
			other => Some((key, other.to_string())),
		})
		.collect()
}

fn any_json_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true
	}))
}

async fn handle_response(response: reqwest::Response) -> Result<CallToolResult, McpError> {
	let status = response.status();
	let bytes = response
		.bytes()
		.await
		.map_err(|err| McpError::internal_error(format!("ELF API response error: {err}"), None))?;
	let parsed = serde_json::from_slice::<Value>(&bytes).unwrap_or_else(|_| {
		let raw = String::from_utf8_lossy(&bytes).to_string();
		serde_json::json!({ "raw": raw })
	});
	if status.is_success() {
		Ok(CallToolResult::structured(parsed))
	} else {
		Ok(CallToolResult::structured_error(parsed))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;

	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
	struct ToolDefinition {
		name: &'static str,
		method: HttpMethod,
		path: &'static str,
		description: &'static str,
		streaming: bool,
	}

	impl ToolDefinition {
		const fn new(
			name: &'static str,
			method: HttpMethod,
			path: &'static str,
			description: &'static str,
		) -> Self {
			Self { name, method, path, description, streaming: true }
		}
	}

	const TOOL_MEMORY_ADD_NOTE: &str = "memory_add_note";
	const TOOL_MEMORY_ADD_EVENT: &str = "memory_add_event";
	const TOOL_MEMORY_SEARCH: &str = "memory_search";
	const TOOL_MEMORY_SEARCH_TIMELINE: &str = "memory_search_timeline";
	const TOOL_MEMORY_SEARCH_DETAILS: &str = "memory_search_details";
	const TOOL_MEMORY_LIST: &str = "memory_list";
	const TOOL_MEMORY_UPDATE: &str = "memory_update";
	const TOOL_MEMORY_DELETE: &str = "memory_delete";

	fn build_tools() -> HashMap<&'static str, ToolDefinition> {
		let tools = [
			ToolDefinition::new(
				TOOL_MEMORY_ADD_NOTE,
				HttpMethod::Post,
				"/v1/memory/add_note",
				"Add memory notes.",
			),
			ToolDefinition::new(
				TOOL_MEMORY_ADD_EVENT,
				HttpMethod::Post,
				"/v1/memory/add_event",
				"Add memory extracted from event messages.",
			),
			ToolDefinition::new(
				TOOL_MEMORY_SEARCH,
				HttpMethod::Post,
				"/v1/memory/search",
				"Search memory notes.",
			),
			ToolDefinition::new(
				TOOL_MEMORY_SEARCH_TIMELINE,
				HttpMethod::Post,
				"/v1/memory/search/timeline",
				"Build a timeline view from a search session.",
			),
			ToolDefinition::new(
				TOOL_MEMORY_SEARCH_DETAILS,
				HttpMethod::Post,
				"/v1/memory/search/details",
				"Fetch full note details for selected ids from a search session.",
			),
			ToolDefinition::new(
				TOOL_MEMORY_LIST,
				HttpMethod::Get,
				"/v1/memory/list",
				"List memory notes.",
			),
			ToolDefinition::new(
				TOOL_MEMORY_UPDATE,
				HttpMethod::Post,
				"/v1/memory/update",
				"Update memory notes.",
			),
			ToolDefinition::new(
				TOOL_MEMORY_DELETE,
				HttpMethod::Post,
				"/v1/memory/delete",
				"Delete memory notes.",
			),
		];

		tools.into_iter().map(|tool| (tool.name, tool)).collect()
	}

	#[test]
	fn registers_all_tools() {
		let tools = build_tools();
		let expected = [
			TOOL_MEMORY_ADD_NOTE,
			TOOL_MEMORY_ADD_EVENT,
			TOOL_MEMORY_SEARCH,
			TOOL_MEMORY_SEARCH_TIMELINE,
			TOOL_MEMORY_SEARCH_DETAILS,
			TOOL_MEMORY_LIST,
			TOOL_MEMORY_UPDATE,
			TOOL_MEMORY_DELETE,
		];

		for name in expected {
			assert!(tools.contains_key(name), "Missing tool registration: {name}.");
		}

		assert_eq!(tools.len(), expected.len(), "Unexpected tool count for MCP registration.");
	}
}
