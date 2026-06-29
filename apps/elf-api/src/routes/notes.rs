use crate::routes::{
	self, AddNoteRequest, AddNoteResponse, ApiError, AppState, DeleteRequest, DeleteResponse,
	ErrorBody, Extension, HeaderMap, Json, JsonRejection, ListRequest, ListResponse,
	MAX_NOTES_PER_INGEST, NoteFetchRequest, NoteFetchResponse, NotePatchRequest,
	NotesIngestRequest, NotesListQuery, Path, PublishNoteRequest, PublishResponseV2, Query,
	QueryRejection, RequestContext, SecurityAuthRole, ShareScope, ShareScopeBody, State,
	StatusCode, UnpublishNoteRequest, UpdateRequest, UpdateResponse, Uuid,
};

#[utoipa::path(
	post,
	path = "/v2/notes/ingest",
	tag = "notes",
	request_body = Value,
	responses(
		(status = 200, description = "Notes were processed.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn notes_ingest(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	payload: Result<Json<NotesIngestRequest>, JsonRejection>,
) -> Result<Json<AddNoteResponse>, ApiError> {
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
	let role = role.map(|Extension(role)| role);

	if payload.scope.trim() == "org_shared" {
		routes::require_admin_for_org_shared_writes(
			state.service.cfg.security.auth_mode.as_str(),
			role,
		)?;
	}
	if payload.notes.len() > MAX_NOTES_PER_INGEST {
		return Err(routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Notes list is too large.",
			Some(vec!["$.notes".to_string()]),
		));
	}

	let response = state
		.service
		.add_note(AddNoteRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope: payload.scope,
			notes: payload.notes,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/notes",
	tag = "notes",
	params(
		("scope" = Option<String>, Query, description = "Optional note scope filter."),
		("status" = Option<String>, Query, description = "Optional note status filter."),
		("type" = Option<String>, Query, description = "Optional note type filter."),
	),
	responses(
		(status = 200, description = "Notes visible to the caller.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn notes_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	query: Result<Query<NotesListQuery>, QueryRejection>,
) -> Result<Json<ListResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Query(query) = query.map_err(|err| {
		tracing::warn!(error = %err, "Invalid query parameters.");

		routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Invalid query parameters.".to_string(),
			None,
		)
	})?;
	let response = state
		.service
		.list(ListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: Some(ctx.agent_id),
			scope: query.scope,
			status: query.status,
			r#type: query.r#type,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/notes/{note_id}",
	tag = "notes",
	params(("note_id" = Uuid, Path, description = "Note ID.")),
	responses(
		(status = 200, description = "Note details.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Note was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn notes_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(note_id): Path<Uuid>,
) -> Result<Json<NoteFetchResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.get_note(NoteFetchRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			note_id,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	patch,
	path = "/v2/notes/{note_id}",
	tag = "notes",
	params(("note_id" = Uuid, Path, description = "Note ID.")),
	request_body = Value,
	responses(
		(status = 200, description = "Note was updated.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Note was not found.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn notes_patch(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(note_id): Path<Uuid>,
	payload: Result<Json<NotePatchRequest>, JsonRejection>,
) -> Result<Json<UpdateResponse>, ApiError> {
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
	let response = state
		.service
		.update(UpdateRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			note_id,
			text: payload.text,
			importance: payload.importance,
			confidence: payload.confidence,
			ttl_days: payload.ttl_days,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	delete,
	path = "/v2/notes/{note_id}",
	tag = "notes",
	params(("note_id" = Uuid, Path, description = "Note ID.")),
	responses(
		(status = 200, description = "Note was deleted.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Note was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn notes_delete(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(note_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.delete(DeleteRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			note_id,
		})
		.await?;

	Ok(Json(response))
}

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
pub(super) async fn notes_publish(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	Path(note_id): Path<Uuid>,
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
	let role = role.map(|Extension(role)| role);

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
pub(super) async fn notes_unpublish(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	Path(note_id): Path<Uuid>,
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
	let role = role.map(|Extension(role)| role);

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
