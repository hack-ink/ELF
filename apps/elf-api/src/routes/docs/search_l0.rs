use crate::routes::{
	self, ApiError, AppState, DOC_STATUSES, DocsSearchL0Body, DocsSearchL0Request,
	DocsSearchL0Response, HeaderMap, Json, JsonRejection, MAX_QUERY_CHARS, RequestContext,
	StatusCode,
};

pub(super) async fn docs_search_l0_inner(
	state: AppState,
	headers: HeaderMap,
	payload: Result<Json<DocsSearchL0Body>, JsonRejection>,
) -> Result<Json<DocsSearchL0Response>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = routes::required_read_profile(&headers)?;
	let Json(mut payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Invalid request payload.",
			None,
		)
	})?;
	let status = payload.status.as_deref().map(str::trim).filter(|status| !status.is_empty());

	if let Some(status) = status {
		let status = status.to_lowercase();

		if !DOC_STATUSES.contains(&status.as_str()) {
			return Err(routes::json_error(
				StatusCode::BAD_REQUEST,
				"INVALID_REQUEST",
				"status must be one of: active|deleted.",
				Some(vec!["$.status".to_string()]),
			));
		}

		payload.status = Some(status);
	}

	let updated_after =
		routes::parse_optional_rfc3339(payload.updated_after.as_ref(), "$.updated_after")?;
	let updated_before =
		routes::parse_optional_rfc3339(payload.updated_before.as_ref(), "$.updated_before")?;
	let ts_gte = routes::parse_optional_rfc3339(payload.ts_gte.as_ref(), "$.ts_gte")?;
	let ts_lte = routes::parse_optional_rfc3339(payload.ts_lte.as_ref(), "$.ts_lte")?;

	if let (Some(ts_gte), Some(ts_lte)) = (ts_gte, ts_lte)
		&& ts_gte >= ts_lte
	{
		return Err(routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"ts_gte must be earlier than ts_lte.",
			Some(vec!["$.ts_gte".to_string(), "$.ts_lte".to_string()]),
		));
	}
	if let (Some(updated_after), Some(updated_before)) = (updated_after, updated_before)
		&& updated_after >= updated_before
	{
		return Err(routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"updated_after must be earlier than updated_before.",
			Some(vec!["$.updated_after".to_string(), "$.updated_before".to_string()]),
		));
	}

	if payload.query.chars().count() > MAX_QUERY_CHARS {
		return Err(routes::json_error(
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
