use crate::routes::{
	self, ApiError, AppState, ErrorBody, Extension, HeaderMap, Json, JsonRejection, Path,
	PublishNoteRequest, PublishResponseV2, RequestContext, SecurityAuthRole, ShareScope,
	ShareScopeBody, State, StatusCode, UnpublishNoteRequest, Uuid,
};

#[utoipa::path(
	post,
	path = "/v2/notes/{note_id}/publish",
	tag = "notes",
	params(("note_id" = Uuid, Path, description = "Note ID.")),
	request_body = Value,
	responses(
		(status = 200, description = "Note was published to a shared space.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Note was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn notes_publish(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	Path(note_id): Path<Uuid>,
	payload: Result<Json<ShareScopeBody>, JsonRejection>,
) -> Result<Json<PublishResponseV2>, ApiError> {
	let role = role.map(|Extension(role)| role);

	notes_publish_inner(state, headers, role, note_id, payload).await
}

#[utoipa::path(
	post,
	path = "/v2/notes/{note_id}/unpublish",
	tag = "notes",
	params(("note_id" = Uuid, Path, description = "Note ID.")),
	request_body = Value,
	responses(
		(status = 200, description = "Note was returned to private scope.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Note was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn notes_unpublish(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	Path(note_id): Path<Uuid>,
	payload: Result<Json<ShareScopeBody>, JsonRejection>,
) -> Result<Json<PublishResponseV2>, ApiError> {
	let role = role.map(|Extension(role)| role);

	notes_unpublish_inner(state, headers, role, note_id, payload).await
}

pub(super) async fn notes_publish_inner(
	state: AppState,
	headers: HeaderMap,
	role: Option<SecurityAuthRole>,
	note_id: Uuid,
	payload: Result<Json<ShareScopeBody>, JsonRejection>,
) -> Result<Json<PublishResponseV2>, ApiError> {
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
	let scope = routes::parse_space(payload.space.as_str())?;

	if matches!(scope, ShareScope::OrgShared) {
		routes::require_admin_for_org_shared_writes(
			state.service.cfg.security.auth_mode.as_str(),
			role,
		)?;
	}

	let response = state
		.service
		.publish_note(PublishNoteRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			note_id,
			scope,
		})
		.await?;

	Ok(Json(PublishResponseV2 {
		note_id: response.note_id,
		space: routes::format_scope(response.scope.as_str())?.to_string(),
	}))
}

pub(super) async fn notes_unpublish_inner(
	state: AppState,
	headers: HeaderMap,
	role: Option<SecurityAuthRole>,
	note_id: Uuid,
	payload: Result<Json<ShareScopeBody>, JsonRejection>,
) -> Result<Json<PublishResponseV2>, ApiError> {
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
	let scope = routes::parse_space(payload.space.as_str())?;

	if matches!(scope, ShareScope::OrgShared) {
		routes::require_admin_for_org_shared_writes(
			state.service.cfg.security.auth_mode.as_str(),
			role,
		)?;
	}

	let response = state
		.service
		.unpublish_note(UnpublishNoteRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			note_id,
		})
		.await?;

	Ok(Json(PublishResponseV2 {
		note_id: response.note_id,
		space: routes::format_scope(response.scope.as_str())?.to_string(),
	}))
}
