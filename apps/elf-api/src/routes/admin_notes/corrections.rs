use crate::routes::{
	self, AdminNoteCorrectionBody, ApiError, AppState, ErrorBody, HeaderMap, Json, JsonRejection,
	MemoryCorrectionRequest, MemoryCorrectionResponse, Path, RequestContext, State, StatusCode,
	Uuid,
};

#[utoipa::path(
	post,
	path = "/v2/admin/notes/{note_id}/corrections",
	tag = "admin",
	params(("note_id" = Uuid, Path, description = "Note ID.")),
	request_body = Value,
	responses(
		(status = 200, description = "Memory correction was applied.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Note was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_note_correction_apply(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(note_id): Path<Uuid>,
	payload: Result<Json<AdminNoteCorrectionBody>, JsonRejection>,
) -> Result<Json<MemoryCorrectionResponse>, ApiError> {
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
		.memory_correction_apply(MemoryCorrectionRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			actor_agent_id: ctx.agent_id,
			note_id,
			action: payload.action,
			reason: payload.reason,
			source_ref: payload.source_ref,
			restore_version_id: payload.restore_version_id,
		})
		.await?;

	Ok(Json(response))
}
