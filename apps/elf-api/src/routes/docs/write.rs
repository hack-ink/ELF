use crate::routes::{
	self, ApiError, AppState, DocsDeleteRequest, DocsDeleteResponse, DocsPutBody, DocsPutRequest,
	DocsPutResponse, ErrorBody, Extension, HeaderMap, Json, JsonRejection, Path, RequestContext,
	SecurityAuthRole, State, StatusCode, Uuid,
};

#[utoipa::path(
	post,
	path = "/v2/docs",
	tag = "docs",
	request_body = Value,
	responses(
		(status = 200, description = "Document was stored.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn docs_put(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	payload: Result<Json<DocsPutBody>, JsonRejection>,
) -> Result<Json<DocsPutResponse>, ApiError> {
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
	let role = role.map(|Extension(role)| role);

	if payload.scope.trim() == "org_shared" {
		routes::require_admin_for_org_shared_writes(
			state.service.cfg.security.auth_mode.as_str(),
			role,
		)?;
	}

	let response = state
		.service
		.docs_put(DocsPutRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope: payload.scope,
			doc_type: payload.doc_type.map(|doc_type| doc_type.as_str().to_string()),
			title: payload.title,
			source_ref: payload.source_ref,
			write_policy: payload.write_policy,
			content: payload.content,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	delete,
	path = "/v2/docs/{doc_id}",
	tag = "docs",
	params(("doc_id" = Uuid, Path, description = "Document ID.")),
	responses(
		(status = 200, description = "Document was deleted.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Document was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn docs_delete(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(doc_id): Path<Uuid>,
) -> Result<Json<DocsDeleteResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.docs_delete(DocsDeleteRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			doc_id,
		})
		.await?;

	Ok(Json(response))
}
