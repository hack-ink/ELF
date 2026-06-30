use crate::routes::{
	ApiError, AppState, ErrorBody, HeaderMap, Json, Path, Query, QueryRejection, RequestContext,
	SearchIndexResponseV2, SearchMode, SearchSessionGetQuery, SearchSessionGetRequest,
	SearchTimelineQuery, SearchTimelineRequest, SearchTimelineResponseV2, State, Uuid,
	search::validation,
};

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
pub(in crate::routes) async fn searches_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(search_id): Path<Uuid>,
	query: Result<Query<SearchSessionGetQuery>, QueryRejection>,
) -> Result<Json<SearchIndexResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Query(query) = query.map_err(validation::invalid_query_parameters)?;
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
pub(in crate::routes) async fn searches_timeline(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(search_id): Path<Uuid>,
	query: Result<Query<SearchTimelineQuery>, QueryRejection>,
) -> Result<Json<SearchTimelineResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Query(query) = query.map_err(validation::invalid_query_parameters)?;
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
