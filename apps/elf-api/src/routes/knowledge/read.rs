use crate::routes::{
	self, ApiError, AppState, ErrorBody, HeaderMap, Json, KnowledgePageGetRequest,
	KnowledgePageLintRequest, KnowledgePageLintResponse, KnowledgePageResponse,
	KnowledgePagesListQuery, KnowledgePagesListRequest, KnowledgePagesListResponse, Path, Query,
	QueryRejection, RequestContext, State, StatusCode, Uuid,
};

#[utoipa::path(
	get,
	path = "/v2/admin/knowledge/pages",
	tag = "knowledge",
	params(
		("page_kind" = Option<String>, Query, description = "Optional page-kind filter."),
		("limit" = Option<u32>, Query, description = "Maximum pages to return."),
	),
	responses(
		(status = 200, description = "Knowledge pages.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn knowledge_pages_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	query: Result<Query<KnowledgePagesListQuery>, QueryRejection>,
) -> Result<Json<KnowledgePagesListResponse>, ApiError> {
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
		.knowledge_pages_list(KnowledgePagesListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			page_kind: query.page_kind,
			limit: query.limit,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/admin/knowledge/pages/{page_id}",
	tag = "knowledge",
	params(("page_id" = Uuid, Path, description = "Knowledge page ID.")),
	responses(
		(status = 200, description = "Knowledge page.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Knowledge page was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn knowledge_page_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(page_id): Path<Uuid>,
) -> Result<Json<KnowledgePageResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.knowledge_page_get(KnowledgePageGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			page_id,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	post,
	path = "/v2/admin/knowledge/pages/{page_id}/lint",
	tag = "knowledge",
	params(("page_id" = Uuid, Path, description = "Knowledge page ID.")),
	responses(
		(status = 200, description = "Knowledge page lint findings.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Knowledge page was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn knowledge_page_lint(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(page_id): Path<Uuid>,
) -> Result<Json<KnowledgePageLintResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.knowledge_page_lint(KnowledgePageLintRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			page_id,
		})
		.await?;

	Ok(Json(response))
}
