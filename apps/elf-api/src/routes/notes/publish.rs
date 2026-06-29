use crate::routes::{
	self, ApiError, AppState, HeaderMap, Json, JsonRejection, PublishNoteRequest,
	PublishResponseV2, RequestContext, SecurityAuthRole, ShareScope, ShareScopeBody, StatusCode,
	UnpublishNoteRequest, Uuid,
};

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
