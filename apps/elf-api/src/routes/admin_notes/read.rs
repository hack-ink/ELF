use crate::routes::{
	ApiError, AppState, ErrorBody, HeaderMap, Json, MemoryHistoryGetRequest, MemoryHistoryResponse,
	NoteProvenanceBundleResponse, NoteProvenanceGetRequest, Path, RequestContext, State, Uuid,
};

#[utoipa::path(
	get,
	path = "/v2/admin/notes/{note_id}/provenance",
	tag = "admin",
	params(("note_id" = Uuid, Path, description = "Note ID.")),
	responses(
		(status = 200, description = "Note provenance bundle.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Note was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_note_provenance_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(note_id): Path<Uuid>,
) -> Result<Json<NoteProvenanceBundleResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.note_provenance_get(NoteProvenanceGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			note_id,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/admin/notes/{note_id}/history",
	tag = "admin",
	params(("note_id" = Uuid, Path, description = "Note ID.")),
	responses(
		(status = 200, description = "Memory history timeline.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Note was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_note_history_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(note_id): Path<Uuid>,
) -> Result<Json<MemoryHistoryResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.memory_history_get(MemoryHistoryGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			note_id,
		})
		.await?;

	Ok(Json(response))
}
