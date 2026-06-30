use crate::routes::{
	self, ApiError, AppState, ErrorBody, HeaderMap, Json, JsonRejection,
	KnowledgePageSearchRequest, KnowledgePageSearchResponse, KnowledgePagesSearchBody,
	RequestContext, State, StatusCode,
};

#[utoipa::path(
	post,
	path = "/v2/admin/knowledge/pages/search",
	tag = "knowledge",
	request_body = Value,
	responses(
		(status = 200, description = "Knowledge page section search results.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn knowledge_pages_search(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<KnowledgePagesSearchBody>, JsonRejection>,
) -> Result<Json<KnowledgePageSearchResponse>, ApiError> {
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
		.knowledge_pages_search(KnowledgePageSearchRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			query: payload.query,
			page_kind: payload.page_kind,
			limit: payload.limit,
		})
		.await?;

	Ok(Json(response))
}
