use super::*;

#[utoipa::path(
	post,
	path = "/v2/searches",
	tag = "search",
	request_body = Value,
	responses(
		(status = 200, description = "Search session was created.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn searches_create(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<SearchCreateRequest>, JsonRejection>,
) -> Result<Json<SearchCreateResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;

	if payload.query.chars().count() > MAX_QUERY_CHARS {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Query is too long.",
			Some(vec!["$.query".to_string()]),
		));
	}
	if payload.top_k.unwrap_or(state.service.cfg.memory.top_k) > MAX_TOP_K {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"top_k is too large.",
			Some(vec!["$.top_k".to_string()]),
		));
	}
	if payload.candidate_k.unwrap_or(state.service.cfg.memory.candidate_k) > MAX_CANDIDATE_K {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"candidate_k is too large.",
			Some(vec!["$.candidate_k".to_string()]),
		));
	}
	if payload.ranking.is_some() {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Ranking overrides are only supported on admin endpoints.".to_string(),
			None,
		));
	}

	let mode = payload.mode;
	let token_id = effective_token_id(state.service.cfg.security.auth_mode.as_str(), &headers);
	let build_request = || SearchRequest {
		tenant_id: ctx.tenant_id,
		project_id: ctx.project_id,
		agent_id: ctx.agent_id,
		token_id: token_id.clone(),
		read_profile,
		query: payload.query.clone(),
		top_k: payload.top_k,
		candidate_k: payload.candidate_k,
		filter: payload.filter.clone(),
		payload_level: payload.payload_level.unwrap_or_default(),
		record_hits: Some(false),
		ranking: None,
	};
	let response = match mode {
		SearchMode::QuickFind => {
			let response = state.service.search_quick(build_request()).await?;

			SearchCreateResponseV2 {
				mode,
				trace_id: response.trace_id,
				search_id: response.search_session_id,
				expires_at: response.expires_at,
				items: response.items,
				trajectory_summary: response.trajectory_summary,
				query_plan: None,
			}
		},
		SearchMode::PlannedSearch => {
			let response = state.service.search_planned(build_request()).await?;

			SearchCreateResponseV2 {
				mode,
				trace_id: response.trace_id,
				search_id: response.search_session_id,
				expires_at: response.expires_at,
				items: response.items,
				trajectory_summary: response.trajectory_summary,
				query_plan: Some(response.query_plan),
			}
		},
	};

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/searches/{search_id}",
	tag = "search",
	params(
		("search_id" = Uuid, Path, description = "Search session ID."),
		("payload_level" = Option<String>, Query, description = "Optional payload level."),
		("top_k" = Option<u32>, Query, description = "Optional result limit override."),
		("touch" = Option<bool>, Query, description = "Whether to extend the session TTL."),
	),
	responses(
		(status = 200, description = "Search session index view.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Search session was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn searches_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(search_id): Path<Uuid>,
	query: Result<Query<SearchSessionGetQuery>, QueryRejection>,
) -> Result<Json<SearchIndexResponseV2>, ApiError> {
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
		.search_session_get(SearchSessionGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			search_session_id: search_id,
			payload_level: query.payload_level.unwrap_or_default(),
			top_k: query.top_k,
			touch: query.touch,
		})
		.await?;
	let mode = if response.query_plan.is_some() {
		SearchMode::PlannedSearch
	} else {
		SearchMode::QuickFind
	};

	Ok(Json(SearchIndexResponseV2 {
		mode,
		trace_id: response.trace_id,
		search_id: response.search_session_id,
		expires_at: response.expires_at,
		items: response.items,
		trajectory_summary: response.trajectory_summary,
		query_plan: response.query_plan,
	}))
}

#[utoipa::path(
	get,
	path = "/v2/searches/{search_id}/timeline",
	tag = "search",
	params(
		("search_id" = Uuid, Path, description = "Search session ID."),
		("payload_level" = Option<String>, Query, description = "Optional payload level."),
		("group_by" = Option<String>, Query, description = "Timeline grouping mode."),
	),
	responses(
		(status = 200, description = "Search session timeline.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Search session was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn searches_timeline(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(search_id): Path<Uuid>,
	query: Result<Query<SearchTimelineQuery>, QueryRejection>,
) -> Result<Json<SearchTimelineResponseV2>, ApiError> {
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
		.search_timeline(SearchTimelineRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			search_session_id: search_id,
			payload_level: query.payload_level.unwrap_or_default(),
			group_by: query.group_by,
		})
		.await?;

	Ok(Json(SearchTimelineResponseV2 {
		search_id: response.search_session_id,
		expires_at: response.expires_at,
		groups: response.groups,
	}))
}

#[utoipa::path(
	post,
	path = "/v2/searches/{search_id}/notes",
	tag = "search",
	params(("search_id" = Uuid, Path, description = "Search session ID.")),
	request_body = Value,
	responses(
		(status = 200, description = "Hydrated search note details.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Search session was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn searches_notes(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(search_id): Path<Uuid>,
	payload: Result<Json<SearchDetailsBody>, JsonRejection>,
) -> Result<Json<SearchDetailsResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;

	if payload.note_ids.len() > MAX_NOTE_IDS_PER_DETAILS {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"note_ids list is too large.",
			Some(vec!["$.note_ids".to_string()]),
		));
	}

	let response = state
		.service
		.search_details(SearchDetailsRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			search_session_id: search_id,
			payload_level: payload.payload_level.unwrap_or_default(),
			note_ids: payload.note_ids,
			record_hits: payload.record_hits,
		})
		.await?;

	Ok(Json(SearchDetailsResponseV2 {
		search_id: response.search_session_id,
		expires_at: response.expires_at,
		results: response.results,
	}))
}

#[utoipa::path(
	post,
	path = "/v2/admin/searches/raw",
	tag = "search",
	request_body = Value,
	responses(
		(status = 200, description = "Raw admin search response.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn searches_raw(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<SearchCreateRequest>, JsonRejection>,
) -> Result<Json<SearchResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;

	if payload.query.chars().count() > MAX_QUERY_CHARS {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Query is too long.",
			Some(vec!["$.query".to_string()]),
		));
	}
	if payload.top_k.unwrap_or(state.service.cfg.memory.top_k) > MAX_TOP_K {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"top_k is too large.",
			Some(vec!["$.top_k".to_string()]),
		));
	}
	if payload.candidate_k.unwrap_or(state.service.cfg.memory.candidate_k) > MAX_CANDIDATE_K {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"candidate_k is too large.",
			Some(vec!["$.candidate_k".to_string()]),
		));
	}

	let request = SearchRequest {
		tenant_id: ctx.tenant_id,
		project_id: ctx.project_id,
		agent_id: ctx.agent_id,
		token_id: effective_token_id(state.service.cfg.security.auth_mode.as_str(), &headers),
		read_profile,
		query: payload.query,
		filter: payload.filter,
		payload_level: payload.payload_level.unwrap_or_default(),
		top_k: payload.top_k,
		candidate_k: payload.candidate_k,
		record_hits: Some(false),
		ranking: payload.ranking,
	};
	let response = match payload.mode {
		SearchMode::QuickFind => state.service.search_raw_quick(request).await?,
		SearchMode::PlannedSearch => {
			let response = state.service.search_raw_planned(request).await?;

			SearchResponse {
				trace_id: response.trace_id,
				items: response.items,
				trajectory_summary: response.trajectory_summary,
			}
		},
	};

	Ok(Json(response))
}
