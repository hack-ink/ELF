use crate::routes::{
	self, ApiError, AppState, CoreBlocksGetRequest, CoreBlocksResponse, EntityMemoryQuery,
	EntityMemoryViewRequest, EntityMemoryViewResponse, ErrorBody, HeaderMap, Json, Query,
	QueryRejection, RequestContext, State, StatusCode,
};

#[utoipa::path(
	get,
	path = "/v2/core-blocks",
	tag = "core_blocks",
	responses(
		(status = 200, description = "Attached core memory blocks.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn core_blocks_get(
	State(state): State<AppState>,
	headers: HeaderMap,
) -> Result<Json<CoreBlocksResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = routes::required_read_profile(&headers)?;
	let response = state
		.service
		.core_blocks_get(CoreBlocksGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/entity-memory",
	tag = "graph",
	params(
		("entity_id" = Option<Uuid>, Query, description = "Graph entity id. Exactly one of entity_id or entity_surface is required."),
		("entity_surface" = Option<String>, Query, description = "Canonical or alias entity surface. Exactly one of entity_id or entity_surface is required."),
	),
	responses(
		(status = 200, description = "Entity-scoped memory authority view.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Entity was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn entity_memory_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	query: Result<Query<EntityMemoryQuery>, QueryRejection>,
) -> Result<Json<EntityMemoryViewResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = routes::required_read_profile(&headers)?;
	let Query(query) = query.map_err(|err| {
		tracing::warn!(error = %err, "Invalid query parameters.");

		ApiError::new(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Invalid query parameters.".to_string(),
			None,
		)
	})?;
	let response = state
		.service
		.entity_memory_view(EntityMemoryViewRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			entity_id: query.entity_id,
			entity_surface: query.entity_surface,
		})
		.await?;

	Ok(Json(response))
}
