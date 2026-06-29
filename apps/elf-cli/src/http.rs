use color_eyre::{Result, eyre};
use reqwest::{Client, Method, RequestBuilder, Response, header::HeaderMap};
use serde_json::Value;

use crate::args::ContextArgs;

pub(crate) struct JsonRequest<'a> {
	pub(crate) method: Method,
	pub(crate) base_url: &'a str,
	pub(crate) path: &'a str,
	pub(crate) token: Option<&'a str>,
	pub(crate) context: Option<&'a ContextArgs>,
	pub(crate) read_profile: Option<&'a str>,
	pub(crate) body: Option<&'a Value>,
}

pub(crate) fn join_url(base_url: &str, path: &str) -> String {
	format!("{}/{}", base_url.trim_end_matches('/'), path.trim_start_matches('/'))
}

pub(crate) fn redact_url(url: &str) -> String {
	url.to_string()
}

pub(crate) fn header_string(headers: &HeaderMap, name: &str) -> Option<String> {
	headers.get(name).and_then(|value| value.to_str().ok()).map(str::to_string)
}

fn add_context_headers(request: RequestBuilder, context: &ContextArgs) -> RequestBuilder {
	request
		.header("X-ELF-Tenant-Id", &context.tenant_id)
		.header("X-ELF-Project-Id", &context.project_id)
		.header("X-ELF-Agent-Id", &context.agent_id)
}

pub(crate) async fn request_json(client: &Client, args: JsonRequest<'_>) -> Result<Value> {
	let mut request = client.request(args.method, join_url(args.base_url, args.path));

	if let Some(token) = args.token {
		request = request.bearer_auth(token);
	}
	if let Some(context) = args.context {
		request = add_context_headers(request, context);
	}
	if let Some(read_profile) = args.read_profile {
		request = request.header("X-ELF-Read-Profile", read_profile);
	}
	if let Some(body) = args.body {
		request = request.json(body);
	}

	parse_json_response(request.send().await?).await
}

pub(crate) async fn request_json_query(
	client: &Client,
	base_url: &str,
	path: &str,
	token: Option<&str>,
	context: &ContextArgs,
	query: &[(&str, String)],
) -> Result<Value> {
	let mut request = client.get(join_url(base_url, path)).query(query);

	if let Some(token) = token {
		request = request.bearer_auth(token);
	}

	request = add_context_headers(request, context);

	parse_json_response(request.send().await?).await
}

async fn parse_json_response(response: Response) -> Result<Value> {
	let status = response.status();
	let request_id = header_string(response.headers(), "x-elf-request-id");
	let text = response.text().await?;

	if !status.is_success() {
		return Err(eyre::eyre!(
			"ELF request failed with HTTP status {status} and request_id {}: {text}",
			request_id.as_deref().unwrap_or("unknown")
		));
	}
	if text.trim().is_empty() {
		return Ok(serde_json::json!({"status": status.as_u16(), "request_id": request_id}));
	}

	serde_json::from_str(&text).map_err(|err| {
		eyre::eyre!(
			"ELF response was not valid JSON for request_id {}: {err}",
			request_id.as_deref().unwrap_or("unknown")
		)
	})
}
