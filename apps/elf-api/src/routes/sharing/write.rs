use crate::routes::{
	self, ApiError, AppState, ErrorBody, Extension, HeaderMap, Json, JsonRejection, Path,
	RequestContext, SecurityAuthRole, ShareScope, SpaceGrantRevokeRequest,
	SpaceGrantRevokeResponse, SpaceGrantUpsertBody, SpaceGrantUpsertRequest,
	SpaceGrantUpsertResponseV2, State, StatusCode,
};

#[utoipa::path(
	post,
	path = "/v2/spaces/{space}/grants",
	tag = "notes",
	params(("space" = String, Path, description = "Shared space name.")),
	request_body = Value,
	responses(
		(status = 200, description = "Space grant was upserted.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn space_grant_upsert(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	Path(space): Path<String>,
	payload: Result<Json<SpaceGrantUpsertBody>, JsonRejection>,
) -> Result<Json<SpaceGrantUpsertResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Invalid request payload.",
			None,
		)
	})?;
	let scope = routes::parse_space(space.as_str())?;
	let role = role.map(|Extension(role)| role);

	if matches!(scope, ShareScope::OrgShared) {
		routes::require_admin_for_org_shared_writes(
			state.service.cfg.security.auth_mode.as_str(),
			role,
		)?;
	}

	let response = state
		.service
		.space_grant_upsert(SpaceGrantUpsertRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope,
			grantee_kind: payload.grantee_kind,
			grantee_agent_id: payload.grantee_agent_id,
		})
		.await?;

	Ok(Json(SpaceGrantUpsertResponseV2 {
		space: routes::format_scope(response.scope.as_str())?.to_string(),
		grantee_kind: response.grantee_kind,
		grantee_agent_id: response.grantee_agent_id,
		granted: response.granted,
	}))
}

#[utoipa::path(
	post,
	path = "/v2/spaces/{space}/grants/revoke",
	tag = "notes",
	params(("space" = String, Path, description = "Shared space name.")),
	request_body = Value,
	responses(
		(status = 200, description = "Space grant was revoked.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn space_grant_revoke(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	Path(space): Path<String>,
	payload: Result<Json<SpaceGrantUpsertBody>, JsonRejection>,
) -> Result<Json<SpaceGrantRevokeResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Invalid request payload.",
			None,
		)
	})?;
	let scope = routes::parse_space(space.as_str())?;
	let role = role.map(|Extension(role)| role);

	if matches!(scope, ShareScope::OrgShared) {
		routes::require_admin_for_org_shared_writes(
			state.service.cfg.security.auth_mode.as_str(),
			role,
		)?;
	}

	let response = state
		.service
		.space_grant_revoke(SpaceGrantRevokeRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope,
			grantee_kind: payload.grantee_kind,
			grantee_agent_id: payload.grantee_agent_id,
		})
		.await?;

	Ok(Json(response))
}
