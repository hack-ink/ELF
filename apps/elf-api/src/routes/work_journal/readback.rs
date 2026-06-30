use crate::routes::{
	self, ApiError, AppState, ErrorBody, HeaderMap, Json, JsonRejection, RequestContext, State,
	StatusCode, WorkJournalSessionReadbackBody, WorkJournalSessionReadbackRequest,
	WorkJournalSessionReadbackResponse,
};

#[utoipa::path(
	post,
	path = "/v2/work-journal/readback",
	tag = "work_journal",
	request_body = Value,
	responses(
		(status = 200, description = "Work Journal session readback.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn work_journal_session_readback(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<WorkJournalSessionReadbackBody>, JsonRejection>,
) -> Result<Json<WorkJournalSessionReadbackResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = routes::required_read_profile(&headers)?;
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
		.work_journal_session_readback(WorkJournalSessionReadbackRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			session_id: payload.session_id,
			families: payload.families,
			limit: payload.limit,
		})
		.await?;

	Ok(Json(response))
}
