use crate::routes::{
	self, ApiError, AppState, DocsGetRequest, DocsGetResponse, ErrorBody, HeaderMap, Json, Path,
	RequestContext, State, Uuid,
};

#[utoipa::path(
	get,
	path = "/v2/docs/{doc_id}",
	tag = "docs",
	params(("doc_id" = Uuid, Path, description = "Document ID.")),
	responses(
		(status = 200, description = "Document was fetched.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Document was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn docs_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(doc_id): Path<Uuid>,
) -> Result<Json<DocsGetResponse>, ApiError> {
	docs_get_inner(state, headers, doc_id).await
}

#[utoipa::path(
	get,
	path = "/v2/admin/docs/{doc_id}",
	tag = "admin",
	params(("doc_id" = Uuid, Path, description = "Document ID.")),
	responses(
		(status = 200, description = "Document was fetched through the admin mirror.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Document was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_docs_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(doc_id): Path<Uuid>,
) -> Result<Json<DocsGetResponse>, ApiError> {
	docs_get_inner(state, headers, doc_id).await
}

async fn docs_get_inner(
	state: AppState,
	headers: HeaderMap,
	doc_id: Uuid,
) -> Result<Json<DocsGetResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = routes::required_read_profile(&headers)?;
	let response = state
		.service
		.docs_get(DocsGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			doc_id,
		})
		.await?;

	Ok(Json(response))
}
