use color_eyre::Result;
use reqwest::{Client, RequestBuilder};
use rmcp::{
	ErrorData,
	handler::server::router::tool::ToolRouter,
	model::{CallToolResult, JsonObject},
};
use serde_json::Value;
use uuid::Uuid;

use crate::app::{
	McpAuthState,
	server::{
		self, HEADER_AGENT_ID, HEADER_AUTHORIZATION, HEADER_PROJECT_ID, HEADER_READ_PROFILE,
		HEADER_REQUEST_ID, HEADER_TENANT_ID,
	},
};
use elf_config::McpContext;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum HttpMethod {
	Get,
	Post,
	Put,
	Patch,
	Delete,
}

#[derive(Clone)]
pub(super) struct ElfContextHeaders {
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
}
impl ElfContextHeaders {
	pub(super) fn new(cfg: &McpContext) -> Self {
		Self {
			tenant_id: cfg.tenant_id.clone(),
			project_id: cfg.project_id.clone(),
			agent_id: cfg.agent_id.clone(),
			read_profile: cfg.read_profile.clone(),
		}
	}
}

#[derive(Clone)]
pub(super) struct ElfMcp {
	pub(super) http_api_base: String,
	pub(super) admin_api_base: String,
	client: Client,
	context: ElfContextHeaders,
	auth_state: McpAuthState,
	pub(super) tool_router: ToolRouter<Self>,
}
impl ElfMcp {
	pub(super) fn new(
		http_api_base: String,
		admin_api_base: String,
		context: ElfContextHeaders,
		auth_state: McpAuthState,
	) -> Self {
		Self {
			http_api_base,
			admin_api_base,
			client: Client::new(),
			context,
			auth_state,
			tool_router: Self::tool_router(),
		}
	}

	pub(super) fn api_base_for_path(&self, path: &str) -> &str {
		if server::is_admin_path(path) { &self.admin_api_base } else { &self.http_api_base }
	}

	fn apply_context_headers(
		&self,
		builder: RequestBuilder,
		read_profile_override: Option<&str>,
		request_id: Uuid,
	) -> RequestBuilder {
		let read_profile = read_profile_override.unwrap_or(self.context.read_profile.as_str());
		let builder = builder
			.header(HEADER_TENANT_ID, self.context.tenant_id.as_str())
			.header(HEADER_PROJECT_ID, self.context.project_id.as_str())
			.header(HEADER_AGENT_ID, self.context.agent_id.as_str())
			.header(HEADER_READ_PROFILE, read_profile);
		let builder = builder.header(HEADER_REQUEST_ID, request_id.to_string());

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
		request_id: Uuid,
	) -> Result<CallToolResult, ErrorData> {
		let url = format!("{}{}", self.api_base_for_path(path), path);
		let response = self
			.apply_context_headers(
				self.client.post(url).json(&body),
				read_profile_override,
				request_id,
			)
			.send()
			.await
			.map_err(|err| {
				ErrorData::internal_error(format!("ELF API request failed: {err}"), None)
			})?;

		server::handle_response(response).await
	}

	async fn forward_patch(
		&self,
		path: &str,
		body: Value,
		read_profile_override: Option<&str>,
		request_id: Uuid,
	) -> Result<CallToolResult, ErrorData> {
		let url = format!("{}{}", self.api_base_for_path(path), path);
		let response = self
			.apply_context_headers(
				self.client.patch(url).json(&body),
				read_profile_override,
				request_id,
			)
			.send()
			.await
			.map_err(|err| {
				ErrorData::internal_error(format!("ELF API request failed: {err}"), None)
			})?;

		server::handle_response(response).await
	}

	async fn forward_put(
		&self,
		path: &str,
		body: Value,
		read_profile_override: Option<&str>,
		request_id: Uuid,
	) -> Result<CallToolResult, ErrorData> {
		let url = format!("{}{}", self.api_base_for_path(path), path);
		let response = self
			.apply_context_headers(
				self.client.put(url).json(&body),
				read_profile_override,
				request_id,
			)
			.send()
			.await
			.map_err(|err| {
				ErrorData::internal_error(format!("ELF API request failed: {err}"), None)
			})?;

		server::handle_response(response).await
	}

	async fn forward_delete(
		&self,
		path: &str,
		read_profile_override: Option<&str>,
		request_id: Uuid,
	) -> Result<CallToolResult, ErrorData> {
		let url = format!("{}{}", self.api_base_for_path(path), path);
		let response = self
			.apply_context_headers(self.client.delete(url), read_profile_override, request_id)
			.send()
			.await
			.map_err(|err| {
				ErrorData::internal_error(format!("ELF API request failed: {err}"), None)
			})?;

		server::handle_response(response).await
	}

	async fn forward_get(
		&self,
		path: &str,
		params: JsonObject,
		read_profile_override: Option<&str>,
		request_id: Uuid,
	) -> Result<CallToolResult, ErrorData> {
		let url = format!("{}{}", self.api_base_for_path(path), path);
		let query = server::params_to_query(params);
		let response = self
			.apply_context_headers(
				self.client.get(url).query(&query),
				read_profile_override,
				request_id,
			)
			.send()
			.await
			.map_err(|err| {
				ErrorData::internal_error(format!("ELF API request failed: {err}"), None)
			})?;

		server::handle_response(response).await
	}

	pub(super) async fn forward(
		&self,
		method: HttpMethod,
		path: &str,
		params: JsonObject,
		read_profile_override: Option<&str>,
	) -> Result<CallToolResult, ErrorData> {
		let request_id = Uuid::new_v4();

		match method {
			HttpMethod::Post =>
				self.forward_post(path, Value::Object(params), read_profile_override, request_id)
					.await,
			HttpMethod::Get =>
				self.forward_get(path, params, read_profile_override, request_id).await,
			HttpMethod::Put =>
				self.forward_put(path, Value::Object(params), read_profile_override, request_id)
					.await,
			HttpMethod::Patch =>
				self.forward_patch(path, Value::Object(params), read_profile_override, request_id)
					.await,
			HttpMethod::Delete =>
				self.forward_delete(path, read_profile_override, request_id).await,
		}
	}
}
