use crate::routes::{
	self, ApiError, AppState, DocsExcerptResponse, DocsExcerptsGetBody, DocsExcerptsGetRequest,
	ErrorBody, HeaderMap, Json, JsonRejection, RequestContext, State, StatusCode,
};

#[utoipa::path(
	post,
	path = "/v2/docs/excerpts",
	tag = "docs",
	request_body = Value,
	responses(
		(status = 200, description = "Document excerpt result.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Document or excerpt was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn docs_excerpts_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<DocsExcerptsGetBody>, JsonRejection>,
) -> Result<Json<DocsExcerptResponse>, ApiError> {
	docs_excerpts_get_inner(state, headers, payload).await
}

#[utoipa::path(
	post,
	path = "/v2/admin/docs/excerpts",
	tag = "admin",
	request_body = Value,
	responses(
		(status = 200, description = "Document excerpt result through the admin mirror.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Document or excerpt was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_docs_excerpts_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<DocsExcerptsGetBody>, JsonRejection>,
) -> Result<Json<DocsExcerptResponse>, ApiError> {
	docs_excerpts_get_inner(state, headers, payload).await
}

async fn docs_excerpts_get_inner(
	state: AppState,
	headers: HeaderMap,
	payload: Result<Json<DocsExcerptsGetBody>, JsonRejection>,
) -> Result<Json<DocsExcerptResponse>, ApiError> {
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
		.docs_excerpts_get(DocsExcerptsGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			doc_id: payload.doc_id,
			level: payload.level,
			chunk_id: payload.chunk_id,
			quote: payload.quote,
			position: payload.position,
			explain: payload.explain,
		})
		.await?;

	Ok(Json(response))
}
