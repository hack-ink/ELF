use crate::routes::{
	self, ApiError, AppState, ErrorBody, HeaderMap, Json, JsonRejection, RequestContext,
	SearchCreateRequest, SearchCreateResponseV2, SearchMode, SearchRequest, State,
	search::validation,
};

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
pub(in crate::routes) async fn searches_create(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<SearchCreateRequest>, JsonRejection>,
) -> Result<Json<SearchCreateResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = routes::required_read_profile(&headers)?;
	let Json(payload) = payload.map_err(validation::invalid_json_payload)?;

	validation::validate_search_create_payload(
		&payload,
		state.service.cfg.memory.top_k,
		state.service.cfg.memory.candidate_k,
	)?;

	let mode = payload.mode;
	let token_id =
		routes::effective_token_id(state.service.cfg.security.auth_mode.as_str(), &headers);
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
