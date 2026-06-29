use super::*;

#[utoipa::path(
	post,
	path = "/v2/admin/knowledge/pages/rebuild",
	tag = "knowledge",
	request_body = Value,
	responses(
		(status = 200, description = "Knowledge page was rebuilt.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn knowledge_page_rebuild(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<KnowledgePageRebuildBody>, JsonRejection>,
) -> Result<Json<KnowledgePageRebuildResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let response = state
		.service
		.knowledge_page_rebuild(KnowledgePageRebuildRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			page_kind: payload.page_kind,
			page_key: payload.page_key,
			title: payload.title,
			doc_ids: payload.doc_ids,
			doc_chunk_ids: payload.doc_chunk_ids,
			note_ids: payload.note_ids,
			event_ids: payload.event_ids,
			relation_ids: payload.relation_ids,
			proposal_ids: payload.proposal_ids,
			provider_metadata: payload.provider_metadata,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	post,
	path = "/v2/admin/knowledge/pages/rebuild-changed-sources",
	tag = "knowledge",
	request_body = Value,
	responses(
		(status = 200, description = "Affected knowledge pages were rebuilt.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn knowledge_pages_watch_rebuild(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<KnowledgePageWatchRebuildBody>, JsonRejection>,
) -> Result<Json<KnowledgePageWatchRebuildResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let changed_sources = payload
		.changed_sources
		.into_iter()
		.map(|source| KnowledgePageChangedSource {
			source_kind: source.source_kind,
			source_id: source.source_id,
		})
		.collect();
	let response = state
		.service
		.knowledge_pages_watch_rebuild(KnowledgePageWatchRebuildRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			changed_sources,
			page_kind: payload.page_kind,
			limit: payload.limit,
			generate_memory_candidates: payload.generate_memory_candidates.unwrap_or(true),
		})
		.await?;

	Ok(Json(response))
}

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
pub(super) async fn knowledge_pages_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	query: Result<Query<KnowledgePagesListQuery>, QueryRejection>,
) -> Result<Json<KnowledgePagesListResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Query(query) = query.map_err(|err| {
		tracing::warn!(error = %err, "Invalid query parameters.");

		json_error(
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
pub(super) async fn knowledge_pages_search(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<KnowledgePagesSearchBody>, JsonRejection>,
) -> Result<Json<KnowledgePageSearchResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
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
pub(super) async fn knowledge_page_get(
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
pub(super) async fn knowledge_page_lint(
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
