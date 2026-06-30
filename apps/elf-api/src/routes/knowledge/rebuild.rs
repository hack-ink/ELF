use crate::routes::{
	self, ApiError, AppState, ErrorBody, HeaderMap, Json, JsonRejection,
	KnowledgePageChangedSource, KnowledgePageRebuildBody, KnowledgePageRebuildRequest,
	KnowledgePageRebuildResponse, KnowledgePageWatchRebuildBody, KnowledgePageWatchRebuildRequest,
	KnowledgePageWatchRebuildResponse, RequestContext, State, StatusCode,
};

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
pub(in crate::routes) async fn knowledge_page_rebuild(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<KnowledgePageRebuildBody>, JsonRejection>,
) -> Result<Json<KnowledgePageRebuildResponse>, ApiError> {
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
pub(in crate::routes) async fn knowledge_pages_watch_rebuild(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<KnowledgePageWatchRebuildBody>, JsonRejection>,
) -> Result<Json<KnowledgePageWatchRebuildResponse>, ApiError> {
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
