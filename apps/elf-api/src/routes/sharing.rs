use crate::routes::{
	self, ApiError, AppState, ErrorBody, Extension, HeaderMap, Json, JsonRejection, Path,
	RequestContext, SecurityAuthRole, ShareScope, SpaceGrantItemV2, SpaceGrantRevokeRequest,
	SpaceGrantRevokeResponse, SpaceGrantUpsertBody, SpaceGrantUpsertRequest,
	SpaceGrantUpsertResponseV2, SpaceGrantsListRequest, SpaceGrantsListResponseV2, State,
	StatusCode,
};

#[utoipa::path(
	get,
	path = "/v2/spaces/{space}/grants",
	tag = "notes",
	params(("space" = String, Path, description = "Shared space name.")),
	responses(
		(status = 200, description = "Space grants.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn space_grants_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(space): Path<String>,
) -> Result<Json<SpaceGrantsListResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let scope = routes::parse_space(space.as_str())?;
	let response = state
		.service
		.space_grants_list(SpaceGrantsListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope,
		})
		.await?;

	Ok(Json(SpaceGrantsListResponseV2 {
		grants: response
			.grants
			.into_iter()
			.map(|item| SpaceGrantItemV2 {
				space: routes::format_space(item.scope).to_string(),
				grantee_kind: item.grantee_kind,
				grantee_agent_id: item.grantee_agent_id,
				granted_by_agent_id: item.granted_by_agent_id,
				granted_at: item.granted_at,
			})
			.collect(),
	}))
}

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
pub(super) async fn space_grant_upsert(
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
pub(super) async fn space_grant_revoke(
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
