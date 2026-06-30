use crate::routes::{
	ApiError, AppState, ErrorBody, HeaderMap, Json, Path, RequestContext, SearchExplainRequest,
	SearchExplainResponse, SearchTrajectoryResponse, State, TraceTrajectoryGetRequest, Uuid,
};

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
pub(in crate::routes) async fn trace_trajectory_get(
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
pub(in crate::routes) async fn trace_item_get(
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
