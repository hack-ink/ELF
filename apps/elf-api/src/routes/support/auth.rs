use crate::routes::{
	AppState, Body, HEADER_AGENT_ID, HEADER_AUTHORIZATION, HEADER_PROJECT_ID, HEADER_READ_PROFILE,
	HEADER_TENANT_ID, HEADER_TRUSTED_TOKEN_ID, HeaderMap, IntoResponse, Next, Request, Response,
	SecurityAuthKey, SecurityAuthRole, State, StatusCode, Uuid,
	support::{
		errors::{self, ApiError},
		request_id,
	},
};

pub(in super::super) fn trusted_token_id(headers: &HeaderMap) -> Option<String> {
	let raw = headers.get(HEADER_TRUSTED_TOKEN_ID)?;
	let value = raw.to_str().ok()?.trim();

	if value.is_empty() { None } else { Some(value.to_string()) }
}

pub(in super::super) fn sanitize_trusted_token_header(headers: &mut HeaderMap) {
	headers.remove(HEADER_TRUSTED_TOKEN_ID);
}

pub(in super::super) fn effective_token_id(auth_mode: &str, headers: &HeaderMap) -> Option<String> {
	match auth_mode.trim() {
		"static_keys" => trusted_token_id(headers),
		_ => None,
	}
}

pub(in super::super) fn bearer_token(headers: &HeaderMap) -> Option<String> {
	let raw = headers.get(HEADER_AUTHORIZATION)?;
	let value = raw.to_str().ok()?.trim();
	let token = value.strip_prefix("Bearer ")?;
	let token = token.trim();

	if token.is_empty() { None } else { Some(token.to_string()) }
}

pub(in super::super) fn resolve_auth_key<'a>(
	headers: &HeaderMap,
	auth_keys: &'a [SecurityAuthKey],
) -> Result<&'a SecurityAuthKey, ApiError> {
	let token = bearer_token(headers).ok_or_else(|| {
		errors::json_error(
			StatusCode::UNAUTHORIZED,
			"UNAUTHORIZED",
			"Authentication required.",
			None,
		)
	})?;

	auth_keys.iter().find(|key| key.token == token).ok_or_else(|| {
		errors::json_error(
			StatusCode::UNAUTHORIZED,
			"UNAUTHORIZED",
			"Authentication required.",
			None,
		)
	})
}

pub(in super::super) fn set_context_header(
	headers: &mut HeaderMap,
	name: &'static str,
	value: &str,
) -> Result<(), ApiError> {
	let header_value = value.parse().map_err(|_| {
		errors::json_error(
			StatusCode::INTERNAL_SERVER_ERROR,
			"INTERNAL_ERROR",
			format!("Invalid configured auth context for {name}."),
			None,
		)
	})?;

	headers.insert(name, header_value);

	Ok(())
}

pub(in super::super) fn apply_auth_key_context(
	headers: &mut HeaderMap,
	key: &SecurityAuthKey,
) -> Result<(), ApiError> {
	let agent_id = key.agent_id.as_deref().ok_or_else(|| {
		errors::json_error(
			StatusCode::FORBIDDEN,
			"FORBIDDEN",
			"Token is not scoped to an agent_id.",
			None,
		)
	})?;

	set_context_header(headers, HEADER_TENANT_ID, key.tenant_id.as_str())?;
	set_context_header(headers, HEADER_PROJECT_ID, key.project_id.as_str())?;
	set_context_header(headers, HEADER_AGENT_ID, agent_id)?;
	set_context_header(headers, HEADER_READ_PROFILE, key.read_profile.as_str())?;
	set_context_header(headers, HEADER_TRUSTED_TOKEN_ID, key.token_id.as_str())?;

	Ok(())
}

pub(in super::super) fn require_admin_for_org_shared_writes(
	auth_mode: &str,
	role: Option<SecurityAuthRole>,
) -> Result<(), ApiError> {
	if auth_mode.trim() != "static_keys" {
		return Ok(());
	}
	if matches!(role, Some(SecurityAuthRole::Admin | SecurityAuthRole::SuperAdmin)) {
		return Ok(());
	}

	Err(errors::json_error(StatusCode::FORBIDDEN, "FORBIDDEN", "Admin token required.", None))
}

pub(in super::super) async fn api_auth_middleware(
	State(state): State<AppState>,
	req: Request<Body>,
	next: Next,
) -> Response {
	let security = &state.service.cfg.security;
	let request_id = match request_id::parse_request_id_from_headers(req.headers()) {
		Ok(request_id) => request_id,
		Err(err) => return request_id::with_request_id(err.into_response(), Uuid::new_v4()).await,
	};
	let mut req = req;

	sanitize_trusted_token_header(req.headers_mut());

	let response = match security.auth_mode.trim() {
		"off" => next.run(req).await,
		"static_keys" => {
			let key = match resolve_auth_key(req.headers(), &security.auth_keys) {
				Ok(key) => key,
				Err(err) =>
					return request_id::with_request_id(err.into_response(), request_id).await,
			};

			req.extensions_mut().insert(key.role);

			if let Err(err) = apply_auth_key_context(req.headers_mut(), key) {
				return request_id::with_request_id(err.into_response(), request_id).await;
			}

			next.run(req).await
		},
		_ => errors::json_error(
			StatusCode::INTERNAL_SERVER_ERROR,
			"INTERNAL_ERROR",
			"Invalid security.auth_mode configuration.",
			None,
		)
		.into_response(),
	};

	request_id::with_request_id(response, request_id).await
}

pub(in super::super) async fn admin_auth_middleware(
	State(state): State<AppState>,
	req: Request<Body>,
	next: Next,
) -> Response {
	let security = &state.service.cfg.security;
	let request_id = match request_id::parse_request_id_from_headers(req.headers()) {
		Ok(request_id) => request_id,
		Err(err) => return request_id::with_request_id(err.into_response(), Uuid::new_v4()).await,
	};
	let mut req = req;

	sanitize_trusted_token_header(req.headers_mut());

	let response = match security.auth_mode.trim() {
		"off" => next.run(req).await,
		"static_keys" => {
			let key = match resolve_auth_key(req.headers(), &security.auth_keys) {
				Ok(key) => key,
				Err(err) =>
					return request_id::with_request_id(err.into_response(), request_id).await,
			};

			req.extensions_mut().insert(key.role);

			if !matches!(key.role, SecurityAuthRole::Admin | SecurityAuthRole::SuperAdmin) {
				return request_id::with_request_id(
					errors::json_error(
						StatusCode::FORBIDDEN,
						"FORBIDDEN",
						"Admin token required.",
						None,
					)
					.into_response(),
					request_id,
				)
				.await;
			}

			if let Err(err) = apply_auth_key_context(req.headers_mut(), key) {
				return request_id::with_request_id(err.into_response(), request_id).await;
			}

			next.run(req).await
		},
		_ => errors::json_error(
			StatusCode::INTERNAL_SERVER_ERROR,
			"INTERNAL_ERROR",
			"Invalid security.auth_mode configuration.",
			None,
		)
		.into_response(),
	};

	request_id::with_request_id(response, request_id).await
}
