use std::{net::SocketAddr, sync::Arc};

use axum::{
	Router,
	body::Body,
	extract::State,
	http::{HeaderMap, Request},
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

use crate::McpAuthState;
use elf_config::McpContext;

const HEADER_TENANT_ID: &str = "X-ELF-Tenant-Id";
const HEADER_PROJECT_ID: &str = "X-ELF-Project-Id";
const HEADER_AGENT_ID: &str = "X-ELF-Agent-Id";
const HEADER_READ_PROFILE: &str = "X-ELF-Read-Profile";
const HEADER_AUTHORIZATION: &str = "Authorization";

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
	auth_state: McpAuthState,
	tool_router: ToolRouter<Self>,
}
impl ElfMcp {
	fn new(api_base: String, context: ElfContextHeaders, auth_state: McpAuthState) -> Self {
		Self {
			api_base,
			client: Client::new(),
			context,
			auth_state,
			tool_router: Self::tool_router(),
		}
	}

	fn apply_context_headers(
		&self,
		builder: RequestBuilder,
		read_profile_override: Option<&str>,
	) -> RequestBuilder {
		let read_profile = read_profile_override.unwrap_or(self.context.read_profile.as_str());
		let builder = builder
			.header(HEADER_TENANT_ID, self.context.tenant_id.as_str())
			.header(HEADER_PROJECT_ID, self.context.project_id.as_str())
			.header(HEADER_AGENT_ID, self.context.agent_id.as_str())
			.header(HEADER_READ_PROFILE, read_profile);

		match &self.auth_state {
			McpAuthState::Off => builder,
			McpAuthState::StaticKeys { bearer_token } =>
				builder.header(HEADER_AUTHORIZATION, format!("Bearer {bearer_token}")),
		}
	}

	async fn forward_post(
		&self,
		path: &str,
		body: Value,
		read_profile_override: Option<&str>,
	) -> Result<CallToolResult, ErrorData> {
		let url = format!("{}{}", self.api_base, path);
		let response = self
			.apply_context_headers(self.client.post(url).json(&body), read_profile_override)
			.send()
			.await
			.map_err(|err| {
				ErrorData::internal_error(format!("ELF API request failed: {err}"), None)
			})?;

		handle_response(response).await
	}

	async fn forward_patch(
		&self,
		path: &str,
		body: Value,
		read_profile_override: Option<&str>,
	) -> Result<CallToolResult, ErrorData> {
		let url = format!("{}{}", self.api_base, path);
		let response = self
			.apply_context_headers(self.client.patch(url).json(&body), read_profile_override)
			.send()
			.await
			.map_err(|err| {
				ErrorData::internal_error(format!("ELF API request failed: {err}"), None)
			})?;

		handle_response(response).await
	}

	async fn forward_delete(
		&self,
		path: &str,
		read_profile_override: Option<&str>,
	) -> Result<CallToolResult, ErrorData> {
		let url = format!("{}{}", self.api_base, path);
		let response = self
			.apply_context_headers(self.client.delete(url), read_profile_override)
			.send()
			.await
			.map_err(|err| {
				ErrorData::internal_error(format!("ELF API request failed: {err}"), None)
			})?;

		handle_response(response).await
	}

	async fn forward_get(
		&self,
		path: &str,
		params: JsonObject,
		read_profile_override: Option<&str>,
	) -> Result<CallToolResult, ErrorData> {
		let url = format!("{}{}", self.api_base, path);
		let query = params_to_query(params);
		let response = self
			.apply_context_headers(self.client.get(url).query(&query), read_profile_override)
			.send()
			.await
			.map_err(|err| {
				ErrorData::internal_error(format!("ELF API request failed: {err}"), None)
			})?;

		handle_response(response).await
	}

	async fn forward(
		&self,
		method: HttpMethod,
		path: &str,
		params: JsonObject,
		read_profile_override: Option<&str>,
	) -> Result<CallToolResult, ErrorData> {
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
	async fn elf_notes_ingest(&self, params: JsonObject) -> Result<CallToolResult, ErrorData> {
		self.forward(HttpMethod::Post, "/v2/notes/ingest", params, None).await
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
		name = "elf_search_quick_create",
		description = "Run a quick search and return a compact index view of results.",
		input_schema = search_quick_create_schema()
	)]
	async fn elf_search_quick_create(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		// read_profile is part of the MCP server configuration and is not client-controlled.
		let _ = take_optional_string(&mut params, "read_profile")?;

		self.forward(HttpMethod::Post, "/v2/search/quick", params, None).await
	}

	#[rmcp::tool(
		name = "elf_search_planned_create",
		description = "Run a planned search and return a compact index view with query_plan.",
		input_schema = search_planned_create_schema()
	)]
	async fn elf_search_planned_create(
		&self,
		mut params: JsonObject,
	) -> Result<CallToolResult, ErrorData> {
		// read_profile is part of the MCP server configuration and is not client-controlled.
		let _ = take_optional_string(&mut params, "read_profile")?;

		self.forward(HttpMethod::Post, "/v2/search/planned", params, None).await
	}

	#[rmcp::tool(
		name = "elf_searches_get",
		description = "Fetch a search session index view by search_id.",
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
		description = "Fetch full note details for selected note_ids from a search session.",
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
	auth_state: McpAuthState,
	mcp_context: &McpContext,
) -> Result<()> {
	let bind_addr: SocketAddr = bind_addr.parse()?;
	let api_base = normalize_api_base(api_base);
	let context = ElfContextHeaders::new(mcp_context);
	let middleware_auth_state = auth_state.clone();
	let client_auth_state = auth_state.clone();
	let session_manager: Arc<LocalSessionManager> = Default::default();
	let service = StreamableHttpService::new(
		move || Ok(ElfMcp::new(api_base.clone(), context.clone(), client_auth_state.clone())),
		session_manager,
		StreamableHttpServerConfig::default(),
	);
	let router = Router::new()
		.fallback_service(service)
		.layer(middleware::from_fn_with_state(middleware_auth_state, mcp_auth_middleware));
	let listener = TcpListener::bind(bind_addr).await?;

	axum::serve(listener, router).await?;

	Ok(())
}

fn is_authorized(headers: &HeaderMap, auth_state: &McpAuthState) -> bool {
	match auth_state {
		McpAuthState::Off => true,
		McpAuthState::StaticKeys { bearer_token } =>
			read_bearer_token(headers).is_some_and(|token| token == bearer_token),
	}
}

fn read_bearer_token(headers: &HeaderMap) -> Option<&str> {
	let raw = headers.get(HEADER_AUTHORIZATION)?;
	let value = raw.to_str().ok()?.trim();
	let token = value.strip_prefix("Bearer ")?.trim();

	if token.is_empty() { None } else { Some(token) }
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

fn take_required_string(params: &mut JsonObject, key: &str) -> Result<String, ErrorData> {
	let value = params
		.remove(key)
		.ok_or_else(|| ErrorData::invalid_params(format!("{key} is required."), None))?;
	let text = value
		.as_str()
		.ok_or_else(|| ErrorData::invalid_params(format!("{key} must be a string."), None))?
		.trim();

	if text.is_empty() {
		return Err(ErrorData::invalid_params(format!("{key} must be non-empty."), None));
	}

	Ok(text.to_string())
}

fn take_optional_string(params: &mut JsonObject, key: &str) -> Result<Option<String>, ErrorData> {
	let Some(value) = params.remove(key) else { return Ok(None) };
	let text = value
		.as_str()
		.ok_or_else(|| ErrorData::invalid_params(format!("{key} must be a string."), None))?
		.trim();

	if text.is_empty() {
		return Err(ErrorData::invalid_params(format!("{key} must be non-empty."), None));
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

fn docs_put_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["scope", "content", "source_ref"],
		"properties": {
			"scope": { "type": "string", "enum": ["agent_private", "project_shared", "org_shared"] },
			"title": { "type": ["string", "null"] },
			"source_ref": { "type": "object", "additionalProperties": true },
			"content": { "type": "string" }
		}
	}))
}

fn docs_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["doc_id"],
		"properties": {
			"doc_id": { "type": "string" }
		}
	}))
}

fn docs_search_l0_schema() -> Arc<JsonObject> {
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

fn docs_excerpts_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["doc_id", "level"],
		"properties": {
			"doc_id": { "type": "string" },
			"level": { "type": "string", "enum": ["L1", "L2"] },
			"chunk_id": { "type": ["string", "null"] },
			"quote": {
				"type": ["object", "null"],
				"additionalProperties": true,
				"required": ["exact"],
				"properties": {
					"exact": { "type": "string" },
					"prefix": { "type": ["string", "null"] },
					"suffix": { "type": ["string", "null"] }
				}
			},
			"position": {
				"type": ["object", "null"],
				"additionalProperties": true,
				"required": ["start", "end"],
				"properties": {
					"start": { "type": "integer" },
					"end": { "type": "integer" }
				}
			}
		}
	}))
}

fn search_create_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["query"],
		"properties": {
			"query": { "type": "string" },
			"payload_level": {
				"type": ["string", "null"],
				"enum": ["l0", "l1", "l2", null]
			},
			"top_k": { "type": ["integer", "null"] },
			"candidate_k": { "type": ["integer", "null"] },
			"read_profile": { "type": ["string", "null"] }
		}
	}))
}

fn search_quick_create_schema() -> Arc<JsonObject> {
	search_create_schema()
}

fn search_planned_create_schema() -> Arc<JsonObject> {
	search_create_schema()
}

fn searches_get_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["search_id"],
		"properties": {
			"search_id": { "type": "string" },
			"payload_level": {
				"type": ["string", "null"],
				"enum": ["l0", "l1", "l2", null]
			},
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
			"payload_level": {
				"type": ["string", "null"],
				"enum": ["l0", "l1", "l2", null]
			},
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
			"payload_level": {
				"type": ["string", "null"],
				"enum": ["l0", "l1", "l2", null]
			},
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

fn notes_publish_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["note_id", "space"],
		"properties": {
			"note_id": { "type": "string" },
			"space": { "type": "string", "enum": ["team_shared", "org_shared"] }
		}
	}))
}

fn notes_unpublish_schema() -> Arc<JsonObject> {
	notes_publish_schema()
}

fn space_grants_list_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["space"],
		"properties": {
			"space": { "type": "string", "enum": ["team_shared", "org_shared"] }
		}
	}))
}

fn space_grant_upsert_schema() -> Arc<JsonObject> {
	Arc::new(rmcp::object!({
		"type": "object",
		"additionalProperties": true,
		"required": ["space", "grantee_kind"],
		"properties": {
			"space": { "type": "string", "enum": ["team_shared", "org_shared"] },
			"grantee_kind": { "type": "string", "enum": ["project", "agent"] },
			"grantee_agent_id": { "type": ["string", "null"] }
		}
	}))
}

fn space_grant_revoke_schema() -> Arc<JsonObject> {
	space_grant_upsert_schema()
}

async fn handle_response(response: reqwest::Response) -> Result<CallToolResult, ErrorData> {
	let status = response.status();
	let bytes = response
		.bytes()
		.await
		.map_err(|err| ErrorData::internal_error(format!("ELF API response error: {err}"), None))?;
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
	State(auth_state): State<McpAuthState>,
	req: Request<Body>,
	next: Next,
) -> axum::response::Response {
	if !is_authorized(req.headers(), &auth_state) {
		return (
			axum::http::StatusCode::UNAUTHORIZED,
			"Authentication required for security.auth_mode=static_keys with a Bearer token.",
		)
			.into_response();
	}

	next.run(req).await
}

#[cfg(test)]
mod tests {
	use axum::http::HeaderMap;
	use std::collections::HashMap;

	use crate::{McpAuthState, server::HttpMethod};

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
				"elf_search_quick_create",
				HttpMethod::Post,
				"/v2/search/quick",
				"Run a quick search and return a compact index view of results.",
			),
			ToolDefinition::new(
				"elf_search_planned_create",
				HttpMethod::Post,
				"/v2/search/planned",
				"Run a planned search and return a compact index view with query_plan.",
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
			ToolDefinition::new(
				"elf_notes_publish",
				HttpMethod::Post,
				"/v2/notes/{note_id}/publish",
				"Publish a note from agent_private into a shared space (team_shared or org_shared).",
			),
			ToolDefinition::new(
				"elf_notes_unpublish",
				HttpMethod::Post,
				"/v2/notes/{note_id}/unpublish",
				"Unpublish a shared note back into agent_private scope.",
			),
			ToolDefinition::new(
				"elf_space_grants_list",
				HttpMethod::Get,
				"/v2/spaces/{space}/grants",
				"List sharing grants for a space (team_shared or org_shared).",
			),
			ToolDefinition::new(
				"elf_space_grant_upsert",
				HttpMethod::Post,
				"/v2/spaces/{space}/grants",
				"Upsert a sharing grant for a space (team_shared or org_shared).",
			),
			ToolDefinition::new(
				"elf_space_grant_revoke",
				HttpMethod::Post,
				"/v2/spaces/{space}/grants/revoke",
				"Revoke a sharing grant for a space (team_shared or org_shared).",
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
			"elf_search_quick_create",
			"elf_search_planned_create",
			"elf_searches_get",
			"elf_searches_timeline",
			"elf_searches_notes",
			"elf_notes_list",
			"elf_notes_get",
			"elf_notes_patch",
			"elf_notes_delete",
			"elf_notes_publish",
			"elf_notes_unpublish",
			"elf_space_grants_list",
			"elf_space_grant_upsert",
			"elf_space_grant_revoke",
		];

		for name in expected {
			assert!(tools.contains_key(name), "Missing tool registration: {name}.");
		}

		assert_eq!(tools.len(), expected.len(), "Unexpected tool count for MCP registration.");
	}

	#[test]
	fn off_mode_allows_requests_without_auth_header() {
		let headers = HeaderMap::new();

		assert!(super::is_authorized(&headers, &McpAuthState::Off));
	}

	#[test]
	fn static_keys_mode_requires_authorization_bearer_header() {
		let mut headers = HeaderMap::new();

		headers
			.insert(super::HEADER_AUTHORIZATION, "Bearer token-a".parse().expect("valid header"));

		assert!(super::is_authorized(
			&headers,
			&McpAuthState::StaticKeys { bearer_token: "token-a".to_string() }
		));
	}

	#[test]
	fn static_keys_mode_rejects_non_bearer_schemes() {
		let mut headers = HeaderMap::new();

		headers
			.insert(super::HEADER_AUTHORIZATION, "bearer token-a".parse().expect("valid header"));

		assert!(!super::is_authorized(
			&headers,
			&McpAuthState::StaticKeys { bearer_token: "token-a".to_string() }
		));
	}
}
