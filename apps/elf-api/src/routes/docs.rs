use super::*;

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
pub(super) async fn docs_put(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	payload: Result<Json<DocsPutBody>, JsonRejection>,
) -> Result<Json<DocsPutResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let role = role.map(|Extension(role)| role);

	if payload.scope.trim() == "org_shared" {
		require_admin_for_org_shared_writes(state.service.cfg.security.auth_mode.as_str(), role)?;
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
pub(super) async fn docs_get(
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
pub(super) async fn admin_docs_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(doc_id): Path<Uuid>,
) -> Result<Json<DocsGetResponse>, ApiError> {
	docs_get_inner(state, headers, doc_id).await
}

pub(super) async fn docs_get_inner(
	state: AppState,
	headers: HeaderMap,
	doc_id: Uuid,
) -> Result<Json<DocsGetResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
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
pub(super) async fn docs_delete(
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

#[utoipa::path(
	post,
	path = "/v2/docs/search/l0",
	tag = "docs",
	request_body = Value,
	responses(
		(status = 200, description = "L0 document search results.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn docs_search_l0(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<DocsSearchL0Body>, JsonRejection>,
) -> Result<Json<DocsSearchL0Response>, ApiError> {
	docs_search_l0_inner(state, headers, payload).await
}

#[utoipa::path(
	post,
	path = "/v2/admin/docs/search/l0",
	tag = "admin",
	request_body = Value,
	responses(
		(status = 200, description = "L0 document search results through the admin mirror.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn admin_docs_search_l0(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<DocsSearchL0Body>, JsonRejection>,
) -> Result<Json<DocsSearchL0Response>, ApiError> {
	docs_search_l0_inner(state, headers, payload).await
}

pub(super) async fn docs_search_l0_inner(
	state: AppState,
	headers: HeaderMap,
	payload: Result<Json<DocsSearchL0Body>, JsonRejection>,
) -> Result<Json<DocsSearchL0Response>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let Json(mut payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let status = payload.status.as_deref().map(str::trim).filter(|status| !status.is_empty());

	if let Some(status) = status {
		let status = status.to_lowercase();

		if !DOC_STATUSES.contains(&status.as_str()) {
			return Err(json_error(
				StatusCode::BAD_REQUEST,
				"INVALID_REQUEST",
				"status must be one of: active|deleted.",
				Some(vec!["$.status".to_string()]),
			));
		}

		payload.status = Some(status);
	}

	let updated_after = parse_optional_rfc3339(payload.updated_after.as_ref(), "$.updated_after")?;
	let updated_before =
		parse_optional_rfc3339(payload.updated_before.as_ref(), "$.updated_before")?;
	let ts_gte = parse_optional_rfc3339(payload.ts_gte.as_ref(), "$.ts_gte")?;
	let ts_lte = parse_optional_rfc3339(payload.ts_lte.as_ref(), "$.ts_lte")?;

	if let (Some(ts_gte), Some(ts_lte)) = (ts_gte, ts_lte)
		&& ts_gte >= ts_lte
	{
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"ts_gte must be earlier than ts_lte.",
			Some(vec!["$.ts_gte".to_string(), "$.ts_lte".to_string()]),
		));
	}
	if let (Some(updated_after), Some(updated_before)) = (updated_after, updated_before)
		&& updated_after >= updated_before
	{
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"updated_after must be earlier than updated_before.",
			Some(vec!["$.updated_after".to_string(), "$.updated_before".to_string()]),
		));
	}

	if payload.query.chars().count() > MAX_QUERY_CHARS {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Query is too long.",
			Some(vec!["$.query".to_string()]),
		));
	}

	let response = state
		.service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			caller_agent_id: ctx.agent_id,
			read_profile,
			query: payload.query,
			scope: payload.scope,
			status: payload.status,
			doc_type: payload.doc_type.map(|doc_type| doc_type.as_str().to_string()),
			sparse_mode: payload.sparse_mode,
			domain: payload.domain,
			repo: payload.repo,
			agent_id: payload.agent_id,
			thread_id: payload.thread_id,
			updated_after: payload.updated_after,
			updated_before: payload.updated_before,
			ts_gte: payload.ts_gte,
			ts_lte: payload.ts_lte,
			top_k: payload.top_k,
			candidate_k: payload.candidate_k,
			explain: payload.explain,
		})
		.await?;

	Ok(Json(response))
}

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
pub(super) async fn docs_excerpts_get(
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
pub(super) async fn admin_docs_excerpts_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<DocsExcerptsGetBody>, JsonRejection>,
) -> Result<Json<DocsExcerptResponse>, ApiError> {
	docs_excerpts_get_inner(state, headers, payload).await
}

pub(super) async fn docs_excerpts_get_inner(
	state: AppState,
	headers: HeaderMap,
	payload: Result<Json<DocsExcerptsGetBody>, JsonRejection>,
) -> Result<Json<DocsExcerptResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
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
