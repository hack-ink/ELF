use super::*;

#[utoipa::path(
	post,
	path = "/v2/admin/consolidation/runs",
	tag = "consolidation",
	request_body = Value,
	responses(
		(status = 200, description = "Consolidation run was created.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn consolidation_run_create(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<ConsolidationRunCreateBody>, JsonRejection>,
) -> Result<Json<ConsolidationRunCreateResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let response = state
		.service
		.consolidation_run_create(ConsolidationRunCreateRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			job_kind: payload.job_kind,
			input_refs: payload.input_refs,
			source_snapshot: payload.source_snapshot,
			lineage: payload.lineage,
			proposals: payload.proposals,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/admin/consolidation/runs",
	tag = "consolidation",
	params(("limit" = Option<u32>, Query, description = "Maximum runs to return.")),
	responses(
		(status = 200, description = "Consolidation runs.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn consolidation_runs_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	query: Result<Query<ConsolidationRunsListQuery>, QueryRejection>,
) -> Result<Json<ConsolidationRunsListResponse>, ApiError> {
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
		.consolidation_runs_list(ConsolidationRunsListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			limit: query.limit,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/admin/consolidation/runs/{run_id}",
	tag = "consolidation",
	params(("run_id" = Uuid, Path, description = "Consolidation run ID.")),
	responses(
		(status = 200, description = "Consolidation run.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Consolidation run was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn consolidation_run_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(run_id): Path<Uuid>,
) -> Result<Json<ConsolidationRunResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.consolidation_run_get(ConsolidationRunGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			run_id,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/admin/consolidation/proposals",
	tag = "consolidation",
	params(
		("run_id" = Option<Uuid>, Query, description = "Optional run filter."),
		("review_state" = Option<String>, Query, description = "Optional review-state filter."),
		("limit" = Option<u32>, Query, description = "Maximum proposals to return."),
	),
	responses(
		(status = 200, description = "Consolidation proposals.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn consolidation_proposals_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	query: Result<Query<ConsolidationProposalsListQuery>, QueryRejection>,
) -> Result<Json<ConsolidationProposalsListResponse>, ApiError> {
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
		.consolidation_proposals_list(ConsolidationProposalsListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			run_id: query.run_id,
			review_state: query.review_state,
			limit: query.limit,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/admin/consolidation/proposals/{proposal_id}",
	tag = "consolidation",
	params(("proposal_id" = Uuid, Path, description = "Consolidation proposal ID.")),
	responses(
		(status = 200, description = "Consolidation proposal.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Consolidation proposal was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn consolidation_proposal_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(proposal_id): Path<Uuid>,
) -> Result<Json<ConsolidationProposalResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.consolidation_proposal_get(ConsolidationProposalGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			proposal_id,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	post,
	path = "/v2/admin/consolidation/proposals/{proposal_id}/review",
	tag = "consolidation",
	params(("proposal_id" = Uuid, Path, description = "Consolidation proposal ID.")),
	request_body = Value,
	responses(
		(status = 200, description = "Consolidation proposal review action was applied.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Consolidation proposal was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn consolidation_proposal_review(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(proposal_id): Path<Uuid>,
	payload: Result<Json<ConsolidationProposalReviewBody>, JsonRejection>,
) -> Result<Json<ConsolidationProposalResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let response = state
		.service
		.consolidation_proposal_review(ConsolidationProposalReviewRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			reviewer_agent_id: ctx.agent_id,
			proposal_id,
			review_action: payload.action,
			review_comment: payload.review_comment,
		})
		.await?;

	Ok(Json(response))
}
