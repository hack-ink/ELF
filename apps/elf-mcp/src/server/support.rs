use super::*;

pub(super) fn is_admin_path(path: &str) -> bool {
	path.starts_with("/v2/admin/")
}

pub(super) fn is_authorized(headers: &HeaderMap, auth_state: &McpAuthState) -> bool {
	match auth_state {
		McpAuthState::Off => true,
		McpAuthState::StaticKeys { bearer_token } =>
			read_bearer_token(headers).is_some_and(|token| token == bearer_token),
	}
}

pub(super) fn read_bearer_token(headers: &HeaderMap) -> Option<&str> {
	let raw = headers.get(HEADER_AUTHORIZATION)?;
	let value = raw.to_str().ok()?.trim();
	let token = value.strip_prefix("Bearer ")?.trim();

	if token.is_empty() { None } else { Some(token) }
}

pub(super) fn normalize_api_base(raw: &str) -> String {
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

pub(super) fn params_to_query(params: JsonObject) -> Vec<(String, String)> {
	params
		.into_iter()
		.filter_map(|(key, value)| match value {
			Value::Null => None,
			Value::String(text) => Some((key, text)),
			other => Some((key, other.to_string())),
		})
		.collect()
}

pub(super) fn take_required_string(
	params: &mut JsonObject,
	key: &str,
) -> Result<String, ErrorData> {
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

pub(super) fn take_optional_string(
	params: &mut JsonObject,
	key: &str,
) -> Result<Option<String>, ErrorData> {
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

pub(super) fn reject_context_override_params(params: &JsonObject) -> Result<(), ErrorData> {
	for key in ["tenant_id", "project_id", "agent_id", "read_profile"] {
		if params.contains_key(key) {
			return Err(ErrorData::invalid_params(
				format!("{key} is configured by the MCP server and must not be supplied."),
				None,
			));
		}
	}

	Ok(())
}

pub(super) async fn handle_response(
	response: reqwest::Response,
) -> Result<CallToolResult, ErrorData> {
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

pub(super) async fn mcp_auth_middleware(
	State(auth_state): State<McpAuthState>,
	req: Request<Body>,
	next: Next,
) -> axum::response::Response {
	if !is_authorized(req.headers(), &auth_state) {
		return (
			StatusCode::UNAUTHORIZED,
			"Authentication required for security.auth_mode=static_keys with a Bearer token.",
		)
			.into_response();
	}

	next.run(req).await
}
