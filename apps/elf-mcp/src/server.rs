use std::{net::SocketAddr, sync::Arc};

use axum::{Router, extract::State, middleware, response::IntoResponse};
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

use elf_config::McpContext;

const HEADER_TENANT_ID: &str = "X-ELF-Tenant-Id";
const HEADER_PROJECT_ID: &str = "X-ELF-Project-Id";
const HEADER_AGENT_ID: &str = "X-ELF-Agent-Id";
const HEADER_READ_PROFILE: &str = "X-ELF-Read-Profile";
const HEADER_AUTHORIZATION: &str = "Authorization";
const HEADER_AUTH_TOKEN: &str = "X-ELF-Auth-Token";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HttpMethod {
	Get,
	Post,
	Patch,
	Delete,
}

#[derive(Clone)]
struct ElfContextHeaders {
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
}
impl ElfContextHeaders {
	fn new(cfg: &McpContext) -> Self {
		Self {
			tenant_id: cfg.tenant_id.clone(),
			project_id: cfg.project_id.clone(),
			agent_id: cfg.agent_id.clone(),
			read_profile: cfg.read_profile.clone(),
		}
	}
}

#[derive(Clone)]
struct ElfMcp {
	api_base: String,
	client: Client,
	context: ElfContextHeaders,
	auth_token: Option<String>,
	tool_router: ToolRouter<Self>,
}
impl ElfMcp {
	fn new(api_base: String, context: ElfContextHeaders, auth_token: Option<String>) -> Self {
		Self {
			api_base,
			client: Client::new(),
			context,
			auth_token,
			tool_router: Self::tool_router(),
		}
	}

	fn apply_context_headers(
		&self,
		builder: reqwest::RequestBuilder,
		read_profile_override: Option<&str>,
	) -> reqwest::RequestBuilder {
		let read_profile = read_profile_override.unwrap_or(self.context.read_profile.as_str());
		let builder = builder
			.header(HEADER_TENANT_ID, self.context.tenant_id.as_str())
			.header(HEADER_PROJECT_ID, self.context.project_id.as_str())
			.header(HEADER_AGENT_ID, self.context.agent_id.as_str())
			.header(HEADER_READ_PROFILE, read_profile);

		if let Some(token) = self.auth_token.as_deref() {
			builder.header(HEADER_AUTHORIZATION, format!("Bearer {token}"))
		} else {
			builder
		}
	}

	async fn forward_post(
		&self,
		path: &str,
		body: Value,
		read_profile_override: Option<&str>,
	) -> Result<CallToolResult, McpError> {
		let url = format!("{}{}", self.api_base, path);
		let response = self
			.apply_context_headers(self.client.post(url).json(&body), read_profile_override)
			.send()
			.await
			.map_err(|err| {
				McpError::internal_error(format!("ELF API request failed: {err}"), None)
			})?;

		handle_response(response).await
	}

	async fn forward_patch(
		&self,
		path: &str,
		body: Value,
		read_profile_override: Option<&str>,
	) -> Result<CallToolResult, McpError> {
		let url = format!("{}{}", self.api_base, path);
		let response = self
			.apply_context_headers(self.client.patch(url).json(&body), read_profile_override)
			.send()
			.await
			.map_err(|err| {
				McpError::internal_error(format!("ELF API request failed: {err}"), None)
			})?;

		handle_response(response).await
	}

	async fn forward_delete(
		&self,
		path: &str,
		read_profile_override: Option<&str>,
	) -> Result<CallToolResult, McpError> {
		let url = format!("{}{}", self.api_base, path);
		let response = self
			.apply_context_headers(self.client.delete(url), read_profile_override)
			.send()
			.await
			.map_err(|err| {
				McpError::internal_error(format!("ELF API request failed: {err}"), None)
			})?;

		handle_response(response).await
	}

	async fn forward_get(
		&self,
		path: &str,
		params: JsonObject,
		read_profile_override: Option<&str>,
	) -> Result<CallToolResult, McpError> {
		let url = format!("{}{}", self.api_base, path);
		let query = params_to_query(params);
		let response = self
			.apply_context_headers(self.client.get(url).query(&query), read_profile_override)
			.send()
			.await
			.map_err(|err| {
				McpError::internal_error(format!("ELF API request failed: {err}"), None)
			})?;

		handle_response(response).await
	}

	async fn forward(
		&self,
		method: HttpMethod,
		path: &str,
		params: JsonObject,
		read_profile_override: Option<&str>,
	) -> Result<CallToolResult, McpError> {
		match method {
			HttpMethod::Post =>
				self.forward_post(path, Value::Object(params), read_profile_override).await,
			HttpMethod::Get => self.forward_get(path, params, read_profile_override).await,
			HttpMethod::Patch =>
				self.forward_patch(path, Value::Object(params), read_profile_override).await,
			HttpMethod::Delete => self.forward_delete(path, read_profile_override).await,
		}
	}
}

#[rmcp::tool_router]
impl ElfMcp {
	#[rmcp::tool(
		name = "elf_notes_ingest",
		description = "Ingest deterministic notes into ELF. This tool never calls an LLM.",
		input_schema = notes_ingest_schema()
	)]
	async fn elf_notes_ingest(&self, params: JsonObject) -> Result<CallToolResult, McpError> {
		self.forward(HttpMethod::Post, "/v2/notes/ingest", params, None).await
	}

	#[rmcp::tool(
		name = "elf_events_ingest",
		description = "Ingest an event by extracting evidence-bound notes using the configured LLM extractor.",
		input_schema = events_ingest_schema()
	)]
	async fn elf_events_ingest(&self, params: JsonObject) -> Result<CallToolResult, McpError> {
		self.forward(HttpMethod::Post, "/v2/events/ingest", params, None).await
	}

	#[rmcp::tool(
		name = "elf_searches_create",
		description = "Create a search session and return a compact index view of results.",
		input_schema = searches_create_schema()
	)]
	async fn elf_searches_create(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, McpError> {
		// read_profile is part of the MCP server configuration and is not client-controlled.
		let _ = take_optional_string(&mut params, "read_profile")?;

		self.forward(HttpMethod::Post, "/v2/searches", params, None).await
	}

	#[rmcp::tool(
		name = "elf_searches_get",
		description = "Fetch a search session index view by search_id.",
		input_schema = searches_get_schema()
	)]
	async fn elf_searches_get(&self, mut params: JsonObject) -> Result<CallToolResult, McpError> {
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
	) -> Result<CallToolResult, McpError> {
		let search_id = take_required_string(&mut params, "search_id")?;
		let path = format!("/v2/searches/{search_id}/timeline");

		self.forward(HttpMethod::Get, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_searches_notes",
		description = "Fetch full note details for selected note_ids from a search session.",
		input_schema = searches_notes_schema()
	)]
	async fn elf_searches_notes(&self, mut params: JsonObject) -> Result<CallToolResult, McpError> {
		let search_id = take_required_string(&mut params, "search_id")?;
		let path = format!("/v2/searches/{search_id}/notes");

		self.forward(HttpMethod::Post, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_notes_list",
		description = "List notes in a tenant and project with optional filters.",
		input_schema = notes_list_schema()
	)]
	async fn elf_notes_list(&self, params: JsonObject) -> Result<CallToolResult, McpError> {
		self.forward(HttpMethod::Get, "/v2/notes", params, None).await
	}

	#[rmcp::tool(
		name = "elf_notes_get",
		description = "Fetch a single note by note_id.",
		input_schema = notes_get_schema()
	)]
	async fn elf_notes_get(&self, mut params: JsonObject) -> Result<CallToolResult, McpError> {
		let note_id = take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/notes/{note_id}");

		self.forward(HttpMethod::Get, &path, JsonObject::new(), None).await
	}

	#[rmcp::tool(
		name = "elf_notes_patch",
		description = "Patch a note by note_id. Only provided fields are updated.",
		input_schema = notes_patch_schema()
	)]
	async fn elf_notes_patch(&self, mut params: JsonObject) -> Result<CallToolResult, McpError> {
		let note_id = take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/notes/{note_id}");

		self.forward(HttpMethod::Patch, &path, params, None).await
	}

	#[rmcp::tool(
		name = "elf_notes_delete",
		description = "Delete a note by note_id.",
		input_schema = notes_get_schema()
	)]
	async fn elf_notes_delete(&self, mut params: JsonObject) -> Result<CallToolResult, McpError> {
		let note_id = take_required_string(&mut params, "note_id")?;
		let path = format!("/v2/notes/{note_id}");

		self.forward(HttpMethod::Delete, &path, JsonObject::new(), None).await
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

pub async fn serve_mcp(
	bind_addr: &str,
	api_base: &str,
	api_auth_token: Option<&str>,
	mcp_context: &McpContext,
) -> Result<()> {
	let bind_addr: SocketAddr = bind_addr.parse()?;
	let api_base = normalize_api_base(api_base);
	let context = ElfContextHeaders::new(mcp_context);
	let api_auth_token = api_auth_token.map(|value| value.to_string());
	let auth_state = api_auth_token.clone();
	let client_token = api_auth_token.clone();
	let session_manager: Arc<LocalSessionManager> = Default::default();
	let service = StreamableHttpService::new(
		move || Ok(ElfMcp::new(api_base.clone(), context.clone(), client_token.clone())),
		session_manager,
		StreamableHttpServerConfig::default(),
	);
	let router = Router::new()
		.fallback_service(service)
		.layer(middleware::from_fn_with_state(auth_state, mcp_auth_middleware));
	let listener = TcpListener::bind(bind_addr).await?;

	axum::serve(listener, router).await?;

	Ok(())
}

fn is_authorized(headers: &axum::http::HeaderMap, expected: Option<&str>) -> bool {
	let Some(expected) = expected else { return true };

	if let Some(raw) = headers.get(HEADER_AUTH_TOKEN)
		&& let Ok(value) = raw.to_str()
		&& value.trim() == expected
	{
		return true;
	}
	if let Some(raw) = headers.get(HEADER_AUTHORIZATION)
		&& let Ok(value) = raw.to_str()
	{
		let value = value.trim();

		if let Some(token) = value.strip_prefix("Bearer ").or_else(|| value.strip_prefix("bearer "))
		{
			return token.trim() == expected;
		}
	}

	false
}

fn normalize_api_base(raw: &str) -> String {
	let trimmed = raw.trim().trim_end_matches('/');
	let (scheme, rest) = if let Some(value) = trimmed.strip_prefix("http://") {
		("http://", value)
	} else if let Some(value) = trimmed.strip_prefix("https://") {
		("https://", value)
	} else {
		("http://", trimmed)
	};
	// elf-mcp runs on the same host as elf-api. If elf-api binds to a wildcard address, use
	// loopback for forwarding.
	let rest = if let Some(value) = rest.strip_prefix("0.0.0.0:") {
		format!("127.0.0.1:{value}")
	} else if let Some(value) = rest.strip_prefix("[::]:") {
		format!("127.0.0.1:{value}")
	} else {
		rest.to_string()
	};

	format!("{scheme}{rest}")
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

fn take_required_string(params: &mut JsonObject, key: &str) -> Result<String, McpError> {
	let value = params
		.remove(key)
		.ok_or_else(|| McpError::invalid_params(format!("{key} is required."), None))?;
	let text = value
		.as_str()
		.ok_or_else(|| McpError::invalid_params(format!("{key} must be a string."), None))?
		.trim();

	if text.is_empty() {
		return Err(McpError::invalid_params(format!("{key} must be non-empty."), None));
	}

	Ok(text.to_string())
}

fn take_optional_string(params: &mut JsonObject, key: &str) -> Result<Option<String>, McpError> {
	let Some(value) = params.remove(key) else { return Ok(None) };
	let text = value
		.as_str()
		.ok_or_else(|| McpError::invalid_params(format!("{key} must be a string."), None))?
		.trim();

	if text.is_empty() {
		return Err(McpError::invalid_params(format!("{key} must be non-empty."), None));
	}

	Ok(Some(text.to_string()))
}

fn notes_ingest_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["scope", "notes"],
		"properties": {
			"scope": { "type": "string" },
			"notes": {
				"type": "array",
				"items": {
					"type": "object",
					"additionalProperties": true,
					"required": ["type", "text", "importance", "confidence", "source_ref"],
					"properties": {
						"type": { "type": "string" },
						"key": { "type": ["string", "null"] },
						"text": { "type": "string" },
						"importance": { "type": "number" },
						"confidence": { "type": "number" },
						"ttl_days": { "type": ["integer", "null"] },
						"source_ref": { "type": "object", "additionalProperties": true }
					}
				}
			}
		}
	}))
}

fn events_ingest_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["messages"],
		"properties": {
			"scope": { "type": ["string", "null"] },
			"dry_run": { "type": ["boolean", "null"] },
			"messages": {
				"type": "array",
				"items": {
					"type": "object",
					"additionalProperties": true,
					"required": ["role", "content"],
					"properties": {
						"role": { "type": "string" },
						"content": { "type": "string" },
						"ts": { "type": ["string", "null"] },
						"msg_id": { "type": ["string", "null"] }
					}
				}
			}
		}
	}))
}

fn searches_create_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["query"],
		"properties": {
			"query": { "type": "string" },
			"top_k": { "type": ["integer", "null"] },
			"candidate_k": { "type": ["integer", "null"] },
			"read_profile": { "type": ["string", "null"] }
		}
	}))
}

fn searches_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["search_id"],
		"properties": {
			"search_id": { "type": "string" },
			"top_k": { "type": ["integer", "null"] },
			"touch": { "type": ["boolean", "null"] }
		}
	}))
}

fn searches_timeline_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["search_id"],
		"properties": {
			"search_id": { "type": "string" },
			"group_by": { "type": ["string", "null"] }
		}
	}))
}

fn searches_notes_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["search_id", "note_ids"],
		"properties": {
			"search_id": { "type": "string" },
			"note_ids": { "type": "array", "items": { "type": "string" } },
			"record_hits": { "type": ["boolean", "null"] }
		}
	}))
}

fn notes_list_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"properties": {
			"scope": { "type": ["string", "null"] },
			"status": { "type": ["string", "null"] },
			"type": { "type": ["string", "null"] }
		}
	}))
}

fn notes_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["note_id"],
		"properties": {
			"note_id": { "type": "string" }
		}
	}))
}

fn notes_patch_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
	"required": ["note_id"],
	"properties": {
		"note_id": { "type": "string" },
		"text": { "type": ["string", "null"] },
		"importance": { "type": ["number", "null"] },
		"confidence": { "type": ["number", "null"] },
		"ttl_days": { "type": ["integer", "null"] }
	}
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

async fn mcp_auth_middleware(
	State(expected): State<Option<String>>,
	req: axum::http::Request<axum::body::Body>,
	next: middleware::Next,
) -> axum::response::Response {
	let expected = expected.as_deref();

	if expected.is_some() && !is_authorized(req.headers(), expected) {
		return (axum::http::StatusCode::UNAUTHORIZED, "Authentication required.").into_response();
	}

	next.run(req).await
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use super::*;

	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

	fn build_tools() -> HashMap<&'static str, ToolDefinition> {
		let tools = [
			ToolDefinition::new(
				"elf_notes_ingest",
				HttpMethod::Post,
				"/v2/notes/ingest",
				"Ingest deterministic notes into ELF. This tool never calls an LLM.",
			),
			ToolDefinition::new(
				"elf_events_ingest",
				HttpMethod::Post,
				"/v2/events/ingest",
				"Ingest an event by extracting evidence-bound notes using the configured LLM extractor.",
			),
			ToolDefinition::new(
				"elf_searches_create",
				HttpMethod::Post,
				"/v2/searches",
				"Create a search session and return a compact index view of results.",
			),
			ToolDefinition::new(
				"elf_searches_get",
				HttpMethod::Get,
				"/v2/searches/{search_id}",
				"Fetch a search session index view by search_id.",
			),
			ToolDefinition::new(
				"elf_searches_timeline",
				HttpMethod::Get,
				"/v2/searches/{search_id}/timeline",
				"Build a timeline view from a search session.",
			),
			ToolDefinition::new(
				"elf_searches_notes",
				HttpMethod::Post,
				"/v2/searches/{search_id}/notes",
				"Fetch full note details for selected note_ids from a search session.",
			),
			ToolDefinition::new(
				"elf_notes_list",
				HttpMethod::Get,
				"/v2/notes",
				"List notes in a tenant and project with optional filters.",
			),
			ToolDefinition::new(
				"elf_notes_get",
				HttpMethod::Get,
				"/v2/notes/{note_id}",
				"Fetch a single note by note_id.",
			),
			ToolDefinition::new(
				"elf_notes_patch",
				HttpMethod::Patch,
				"/v2/notes/{note_id}",
				"Patch a note by note_id. Only provided fields are updated.",
			),
			ToolDefinition::new(
				"elf_notes_delete",
				HttpMethod::Delete,
				"/v2/notes/{note_id}",
				"Delete a note by note_id.",
			),
		];

		tools.into_iter().map(|tool| (tool.name, tool)).collect()
	}

	#[test]
	fn registers_all_tools() {
		let tools = build_tools();
		let expected = [
			"elf_notes_ingest",
			"elf_events_ingest",
			"elf_searches_create",
			"elf_searches_get",
			"elf_searches_timeline",
			"elf_searches_notes",
			"elf_notes_list",
			"elf_notes_get",
			"elf_notes_patch",
			"elf_notes_delete",
		];

		for name in expected {
			assert!(tools.contains_key(name), "Missing tool registration: {name}.");
		}

		assert_eq!(tools.len(), expected.len(), "Unexpected tool count for MCP registration.");
	}
}
