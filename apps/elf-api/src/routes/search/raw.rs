use crate::routes::{
	self, ApiError, AppState, ErrorBody, HeaderMap, Json, JsonRejection, RequestContext,
	SearchCreateRequest, SearchMode, SearchRequest, SearchResponse, State, search::validation,
};

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
pub(in crate::routes) async fn searches_raw(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<SearchCreateRequest>, JsonRejection>,
) -> Result<Json<SearchResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = routes::required_read_profile(&headers)?;
	let Json(payload) = payload.map_err(validation::invalid_json_payload)?;

	validation::validate_search_raw_payload(
		&payload,
		state.service.cfg.memory.top_k,
		state.service.cfg.memory.candidate_k,
	)?;

	let request = SearchRequest {
		tenant_id: ctx.tenant_id,
		project_id: ctx.project_id,
		agent_id: ctx.agent_id,
		token_id: routes::effective_token_id(
			state.service.cfg.security.auth_mode.as_str(),
			&headers,
		),
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
