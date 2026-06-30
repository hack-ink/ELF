use crate::routes::{
	self, AdminGraphPredicateAliasAddBody, AdminGraphPredicateAliasAddRequest,
	AdminGraphPredicateAliasesListRequest, AdminGraphPredicateAliasesResponse,
	AdminGraphPredicatePatchBody, AdminGraphPredicatePatchRequest, AdminGraphPredicateResponse,
	AdminGraphPredicatesListQuery, AdminGraphPredicatesListRequest,
	AdminGraphPredicatesListResponse, ApiError, AppState, ErrorBody, HeaderMap, Json,
	JsonRejection, Path, Query, QueryRejection, RequestContext, State, StatusCode, Uuid,
};

#[utoipa::path(
	get,
	path = "/v2/admin/graph/predicates",
	tag = "graph",
	params(("scope" = Option<String>, Query, description = "Predicate scope filter.")),
	responses(
		(status = 200, description = "Graph predicates.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_graph_predicates_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	query: Result<Query<AdminGraphPredicatesListQuery>, QueryRejection>,
) -> Result<Json<AdminGraphPredicatesListResponse>, ApiError> {
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
		.admin_graph_predicates_list(AdminGraphPredicatesListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope: query.scope,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	patch,
	path = "/v2/admin/graph/predicates/{predicate_id}",
	tag = "graph",
	params(("predicate_id" = Uuid, Path, description = "Predicate ID.")),
	request_body = Value,
	responses(
		(status = 200, description = "Graph predicate was updated.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Predicate was not found.", body = ErrorBody),
		(status = 409, description = "Predicate update conflicted.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_graph_predicate_patch(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(predicate_id): Path<Uuid>,
	payload: Result<Json<AdminGraphPredicatePatchBody>, JsonRejection>,
) -> Result<Json<AdminGraphPredicateResponse>, ApiError> {
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
	let token_id =
		routes::effective_token_id(state.service.cfg.security.auth_mode.as_str(), &headers);
	let response = state
		.service
		.admin_graph_predicate_patch(AdminGraphPredicatePatchRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			token_id,
			predicate_id,
			status: payload.status,
			cardinality: payload.cardinality,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	post,
	path = "/v2/admin/graph/predicates/{predicate_id}/aliases",
	tag = "graph",
	params(("predicate_id" = Uuid, Path, description = "Predicate ID.")),
	request_body = Value,
	responses(
		(status = 200, description = "Graph predicate alias was added.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Predicate was not found.", body = ErrorBody),
		(status = 409, description = "Predicate update conflicted.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_graph_predicate_alias_add(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(predicate_id): Path<Uuid>,
	payload: Result<Json<AdminGraphPredicateAliasAddBody>, JsonRejection>,
) -> Result<Json<AdminGraphPredicateAliasesResponse>, ApiError> {
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
	let token_id =
		routes::effective_token_id(state.service.cfg.security.auth_mode.as_str(), &headers);
	let response = state
		.service
		.admin_graph_predicate_alias_add(AdminGraphPredicateAliasAddRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			token_id,
			predicate_id,
			alias: payload.alias,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/admin/graph/predicates/{predicate_id}/aliases",
	tag = "graph",
	params(("predicate_id" = Uuid, Path, description = "Predicate ID.")),
	responses(
		(status = 200, description = "Graph predicate aliases.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Predicate was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_graph_predicate_aliases_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(predicate_id): Path<Uuid>,
) -> Result<Json<AdminGraphPredicateAliasesResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.admin_graph_predicate_aliases_list(AdminGraphPredicateAliasesListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			predicate_id,
		})
		.await?;

	Ok(Json(response))
}
