use crate::routes::{
	self, ApiError, AppState, ContextPackBody, ContextPackRequest, ContextPackResponse, ErrorBody,
	HeaderMap, Json, JsonRejection, RequestContext, State, StatusCode,
};

#[utoipa::path(
	post,
	path = "/v2/context-packs",
	tag = "context-pack",
	request_body = Value,
	responses(
		(status = 200, description = "Read-time Context Pack v1.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn context_pack_build(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<ContextPackBody>, JsonRejection>,
) -> Result<Json<ContextPackResponse>, ApiError> {
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
		.context_pack_build(ContextPackRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			task: payload.task,
			title: payload.title,
			description: payload.description,
			trace_id: payload.trace_id,
			query: payload.query,
			docs_query: payload.docs_query,
			knowledge_query: payload.knowledge_query,
			graph_subject: payload.graph_subject,
			graph_predicate: payload.graph_predicate,
			include_dreaming: payload.include_dreaming,
			limit: payload.limit,
			debug_overrides: None,
		})
		.await?;

	Ok(Json(response))
}
