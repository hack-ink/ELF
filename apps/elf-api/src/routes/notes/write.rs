use crate::routes::{
	self, ApiError, AppState, DeleteRequest, DeleteResponse, ErrorBody, HeaderMap, Json,
	JsonRejection, NotePatchRequest, Path, RequestContext, State, StatusCode, UpdateRequest,
	UpdateResponse, Uuid,
};

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
pub(in crate::routes) async fn notes_patch(
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
pub(in crate::routes) async fn notes_delete(
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
