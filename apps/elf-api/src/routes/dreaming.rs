use super::*;

#[utoipa::path(
	get,
	path = "/v2/admin/dreaming/review-queue",
	tag = "dreaming",
	params(
		("run_id" = Option<Uuid>, Query, description = "Optional consolidation run filter."),
		("review_state" = Option<String>, Query, description = "Optional review-state filter."),
		("limit" = Option<u32>, Query, description = "Maximum queue items to return."),
	),
	responses(
		(status = 200, description = "Dreaming review queue items.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn dreaming_review_queue(
	State(state): State<AppState>,
	headers: HeaderMap,
	query: Result<Query<DreamingReviewQueueQuery>, QueryRejection>,
) -> Result<Json<DreamingReviewQueueResponse>, ApiError> {
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
		.dreaming_review_queue(DreamingReviewQueueRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			run_id: query.run_id,
			review_state: query.review_state,
			limit: query.limit,
		})
		.await?;

	Ok(Json(response))
}
