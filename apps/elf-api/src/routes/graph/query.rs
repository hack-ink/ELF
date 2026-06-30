use crate::routes::{
	self, ApiError, AppState, ErrorBody, GraphQueryBody, GraphQueryRequest, GraphQueryResponse,
	GraphReportBody, GraphReportRequest, GraphReportResponse, HeaderMap, Json, JsonRejection,
	RequestContext, State, StatusCode,
};

#[utoipa::path(
	post,
	path = "/v2/graph/query",
	tag = "graph",
	request_body = Value,
	responses(
		(status = 200, description = "Graph facts matching the query.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn graph_query(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<GraphQueryBody>, JsonRejection>,
) -> Result<Json<GraphQueryResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = routes::required_read_profile(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Invalid request payload.",
			None,
		)
	})?;
	let as_of = routes::parse_optional_rfc3339(payload.as_of.as_ref(), "$.as_of")?;
	let response = state
		.service
		.graph_query(GraphQueryRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			subject: payload.subject,
			predicate: payload.predicate,
			scopes: payload.scopes,
			as_of,
			limit: payload.limit,
			explain: payload.explain,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	post,
	path = "/v2/graph/report",
	tag = "graph",
	request_body = Value,
	responses(
		(status = 200, description = "Source-backed graph topic-map report.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn graph_report(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<GraphReportBody>, JsonRejection>,
) -> Result<Json<GraphReportResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = routes::required_read_profile(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Invalid request payload.",
			None,
		)
	})?;
	let as_of = routes::parse_optional_rfc3339(payload.as_of.as_ref(), "$.as_of")?;
	let response = state
		.service
		.graph_report(GraphReportRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			subject: payload.subject,
			predicate: payload.predicate,
			scopes: payload.scopes,
			as_of,
			limit: payload.limit,
			explain: payload.explain,
		})
		.await?;

	Ok(Json(response))
}
