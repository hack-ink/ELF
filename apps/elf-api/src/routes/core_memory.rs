use super::*;

#[utoipa::path(
	get,
	path = "/v2/core-blocks",
	tag = "core_blocks",
	responses(
		(status = 200, description = "Attached core memory blocks.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn core_blocks_get(
	State(state): State<AppState>,
	headers: HeaderMap,
) -> Result<Json<CoreBlocksResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let response = state
		.service
		.core_blocks_get(CoreBlocksGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/entity-memory",
	tag = "graph",
	params(
		("entity_id" = Option<Uuid>, Query, description = "Graph entity id. Exactly one of entity_id or entity_surface is required."),
		("entity_surface" = Option<String>, Query, description = "Canonical or alias entity surface. Exactly one of entity_id or entity_surface is required."),
	),
	responses(
		(status = 200, description = "Entity-scoped memory authority view.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Entity was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn entity_memory_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	query: Result<Query<EntityMemoryQuery>, QueryRejection>,
) -> Result<Json<EntityMemoryViewResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let Query(query) = query.map_err(|err| {
		tracing::warn!(error = %err, "Invalid query parameters.");

		ApiError::new(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Invalid query parameters.".to_string(),
			None,
		)
	})?;
	let response = state
		.service
		.entity_memory_view(EntityMemoryViewRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			entity_id: query.entity_id,
			entity_surface: query.entity_surface,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	post,
	path = "/v2/admin/core-blocks",
	tag = "core_blocks",
	request_body = Value,
	responses(
		(status = 200, description = "Core block was stored.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 409, description = "Core block conflict.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn admin_core_block_upsert(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	payload: Result<Json<CoreBlockUpsertBody>, JsonRejection>,
) -> Result<Json<CoreBlockUpsertResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let role = role.map(|Extension(role)| role);

	if payload.scope.trim() == "org_shared" {
		require_admin_for_org_shared_writes(state.service.cfg.security.auth_mode.as_str(), role)?;
	}

	let response = state
		.service
		.core_block_upsert(CoreBlockUpsertRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			block_id: payload.block_id,
			scope: payload.scope,
			key: payload.key,
			title: payload.title,
			content: payload.content,
			source_ref: payload.source_ref,
			reason: payload.reason,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	post,
	path = "/v2/admin/core-blocks/{block_id}/attachments",
	tag = "core_blocks",
	params(("block_id" = Uuid, Path, description = "Core block ID.")),
	request_body = Value,
	responses(
		(status = 200, description = "Core block was attached.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Core block was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn admin_core_block_attach(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(block_id): Path<Uuid>,
	payload: Result<Json<CoreBlockAttachBody>, JsonRejection>,
) -> Result<Json<CoreBlockAttachResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let response = state
		.service
		.core_block_attach(CoreBlockAttachRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			block_id,
			target_agent_id: payload.target_agent_id,
			read_profile: payload.read_profile,
			reason: payload.reason,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	delete,
	path = "/v2/admin/core-blocks/attachments/{attachment_id}",
	tag = "core_blocks",
	params(("attachment_id" = Uuid, Path, description = "Core block attachment ID.")),
	responses(
		(status = 200, description = "Core block attachment was detached.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn admin_core_block_detach(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(attachment_id): Path<Uuid>,
) -> Result<Json<CoreBlockDetachResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.core_block_detach(CoreBlockDetachRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			attachment_id,
			reason: None,
		})
		.await?;

	Ok(Json(response))
}
