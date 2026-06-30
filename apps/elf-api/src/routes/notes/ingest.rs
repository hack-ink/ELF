use crate::routes::{
	self, AddNoteRequest, AddNoteResponse, ApiError, AppState, ErrorBody, Extension, HeaderMap,
	Json, JsonRejection, MAX_NOTES_PER_INGEST, NotesIngestRequest, RequestContext,
	SecurityAuthRole, State, StatusCode,
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
pub(in crate::routes) async fn notes_ingest(
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
