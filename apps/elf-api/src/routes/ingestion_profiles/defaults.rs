use crate::routes::{
	self, AdminIngestionProfileDefaultGetRequest, AdminIngestionProfileDefaultResponse,
	AdminIngestionProfileDefaultResponseV2, AdminIngestionProfileDefaultSetBody,
	AdminIngestionProfileDefaultSetRequest, ApiError, AppState, ErrorBody, HeaderMap, Json,
	JsonRejection, RequestContext, State, StatusCode,
};

#[utoipa::path(
	get,
	path = "/v2/admin/events/ingestion-profiles/default",
	tag = "admin",
	responses(
		(
			status = 200,
			description = "Default add_event ingestion profile pointer.",
			body = AdminIngestionProfileDefaultResponseV2,
		),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_ingestion_profile_default_get(
	State(state): State<AppState>,
	headers: HeaderMap,
) -> Result<Json<AdminIngestionProfileDefaultResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.admin_ingestion_profile_default_get(AdminIngestionProfileDefaultGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	put,
	path = "/v2/admin/events/ingestion-profiles/default",
	tag = "admin",
	request_body = AdminIngestionProfileDefaultSetBody,
	responses(
		(
			status = 200,
			description = "Default add_event ingestion profile pointer was updated.",
			body = AdminIngestionProfileDefaultResponseV2,
		),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Profile was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_ingestion_profile_default_set(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<AdminIngestionProfileDefaultSetBody>, JsonRejection>,
) -> Result<Json<AdminIngestionProfileDefaultResponse>, ApiError> {
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
		.admin_ingestion_profile_default_set(AdminIngestionProfileDefaultSetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			profile_id: payload.profile_id,
			version: payload.version,
		})
		.await?;

	Ok(Json(response))
}
