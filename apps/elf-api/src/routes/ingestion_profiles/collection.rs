use crate::routes::{
	self, AdminIngestionProfileCreateBody, AdminIngestionProfileCreateRequest,
	AdminIngestionProfileListRequest, AdminIngestionProfileResponse,
	AdminIngestionProfilesListResponse, ApiError, AppState, ErrorBody, HeaderMap, Json,
	JsonRejection, RequestContext, State, StatusCode,
};

#[utoipa::path(
	get,
	path = "/v2/admin/events/ingestion-profiles",
	tag = "admin",
	responses(
		(status = 200, description = "Ingestion profile versions.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_ingestion_profiles_list(
	State(state): State<AppState>,
	headers: HeaderMap,
) -> Result<Json<AdminIngestionProfilesListResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.admin_ingestion_profiles_list(AdminIngestionProfileListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	post,
	path = "/v2/admin/events/ingestion-profiles",
	tag = "admin",
	request_body = Value,
	responses(
		(status = 200, description = "Ingestion profile version was created.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_ingestion_profile_create(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<AdminIngestionProfileCreateBody>, JsonRejection>,
) -> Result<Json<AdminIngestionProfileResponse>, ApiError> {
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
	let response = state
		.service
		.admin_ingestion_profile_create(AdminIngestionProfileCreateRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			profile_id: payload.profile_id,
			version: payload.version,
			profile: payload.profile,
			created_by: payload.created_by,
		})
		.await?;

	Ok(Json(response))
}
