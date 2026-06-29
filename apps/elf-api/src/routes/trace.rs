use crate::routes::{
	self, ApiError, AppState, ErrorBody, HeaderMap, Json, Path, Query, QueryRejection,
	RequestContext, SearchExplainRequest, SearchExplainResponse, SearchTrajectoryResponse, State,
	StatusCode, TraceBundleGetQuery, TraceBundleGetRequest, TraceBundleResponse, TraceGetRequest,
	TraceGetResponse, TraceRecentListQuery, TraceRecentListRequest, TraceRecentListResponse,
	TraceTrajectoryGetRequest, Uuid,
};

#[utoipa::path(
	get,
	path = "/v2/admin/traces/{trace_id}",
	tag = "admin",
	params(("trace_id" = Uuid, Path, description = "Search trace ID.")),
	responses(
		(status = 200, description = "Search trace bundle without full stage internals.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Trace was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn trace_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(trace_id): Path<Uuid>,
) -> Result<Json<TraceGetResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.trace_get(TraceGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			trace_id,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/admin/traces/recent",
	tag = "admin",
	params(
		("limit" = Option<u32>, Query, description = "Page size."),
		("cursor_created_at" = Option<String>, Query, description = "Created-at page cursor."),
		("cursor_trace_id" = Option<Uuid>, Query, description = "Trace ID page cursor."),
		("agent_id" = Option<String>, Query, description = "Optional trace creator filter."),
		("read_profile" = Option<String>, Query, description = "Optional read profile filter."),
		("created_after" = Option<String>, Query, description = "Strict lower created_at bound."),
		("created_before" = Option<String>, Query, description = "Strict upper created_at bound."),
	),
	responses(
		(status = 200, description = "Recent search traces.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn trace_recent_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	query: Result<Query<TraceRecentListQuery>, QueryRejection>,
) -> Result<Json<TraceRecentListResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Query(query) = query.map_err(|err| {
		tracing::warn!(error = %err, "Invalid query parameters.");

		routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Invalid query parameters.".to_string(),
			None,
		)
	})?;
	let cursor_created_at =
		routes::parse_optional_rfc3339(query.cursor_created_at.as_ref(), "$.cursor_created_at")?;
	let cursor_trace_id = query.cursor_trace_id;
	let created_after =
		routes::parse_optional_rfc3339(query.created_after.as_ref(), "$.created_after")?;
	let created_before =
		routes::parse_optional_rfc3339(query.created_before.as_ref(), "$.created_before")?;

	if cursor_created_at.is_some() != cursor_trace_id.is_some() {
		return Err(routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"cursor_created_at and cursor_trace_id must be both set or both omitted.".to_string(),
			Some(vec!["$.cursor_created_at".to_string(), "$.cursor_trace_id".to_string()]),
		));
	}

	let response = state
		.service
		.trace_recent_list(TraceRecentListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			limit: query.limit,
			cursor_created_at,
			cursor_trace_id,
			agent_id_filter: query.agent_id,
			read_profile: query.read_profile,
			created_after,
			created_before,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/admin/trajectories/{trace_id}",
	tag = "admin",
	params(("trace_id" = Uuid, Path, description = "Search trace ID.")),
	responses(
		(status = 200, description = "Search trace retrieval trajectory.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Trace was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn trace_trajectory_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(trace_id): Path<Uuid>,
) -> Result<Json<SearchTrajectoryResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.trace_trajectory_get(TraceTrajectoryGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			trace_id,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/admin/trace-items/{item_id}",
	tag = "admin",
	params(("item_id" = Uuid, Path, description = "Trace item/result handle ID.")),
	responses(
		(status = 200, description = "Search trace item explain payload.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Trace item was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn trace_item_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(item_id): Path<Uuid>,
) -> Result<Json<SearchExplainResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.search_explain(SearchExplainRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			result_handle: item_id,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/admin/traces/{trace_id}/bundle",
	tag = "admin",
	params(
		("trace_id" = Uuid, Path, description = "Search trace ID."),
		("mode" = Option<String>, Query, description = "bounded or full."),
		("stage_items_limit" = Option<u32>, Query, description = "Maximum stage items."),
		("candidates_limit" = Option<u32>, Query, description = "Maximum candidate snapshot items."),
	),
	responses(
		(status = 200, description = "Search trace bundle.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Trace was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn trace_bundle_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(trace_id): Path<Uuid>,
	query: Result<Query<TraceBundleGetQuery>, QueryRejection>,
) -> Result<Json<TraceBundleResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Query(query) = query.map_err(|err| {
		tracing::warn!(error = %err, "Invalid query parameters.");

		routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Invalid query parameters.".to_string(),
			None,
		)
	})?;
	let response = state
		.service
		.trace_bundle_get(TraceBundleGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			trace_id,
			mode: query.mode.unwrap_or_default(),
			stage_items_limit: query.stage_items_limit,
			candidates_limit: query.candidates_limit,
		})
		.await?;

	Ok(Json(response))
}
