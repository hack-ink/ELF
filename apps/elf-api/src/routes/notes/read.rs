use crate::routes::{
	self, ApiError, AppState, ErrorBody, HeaderMap, Json, ListRequest, ListResponse,
	NoteFetchRequest, NoteFetchResponse, NotesListQuery, Path, Query, QueryRejection,
	RequestContext, State, StatusCode, Uuid,
};

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
pub(in crate::routes) async fn notes_list(
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
pub(in crate::routes) async fn notes_get(
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
