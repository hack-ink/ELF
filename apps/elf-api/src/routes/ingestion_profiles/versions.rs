use crate::routes::{
	self, AdminIngestionProfileGetQuery, AdminIngestionProfileGetRequest,
	AdminIngestionProfileResponse, AdminIngestionProfileVersionsListRequest,
	AdminIngestionProfileVersionsListResponse, ApiError, AppState, ErrorBody, HeaderMap, Json,
	Path, Query, QueryRejection, RequestContext, State, StatusCode,
};

#[utoipa::path(
	get,
	path = "/v2/admin/events/ingestion-profiles/{profile_id}",
	tag = "admin",
	params(
		("profile_id" = String, Path, description = "Ingestion profile ID."),
		("version" = Option<i32>, Query, description = "Optional profile version."),
	),
	responses(
		(status = 200, description = "Ingestion profile version.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 404, description = "Profile was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_ingestion_profile_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(profile_id): Path<String>,
	query: Result<Query<AdminIngestionProfileGetQuery>, QueryRejection>,
) -> Result<Json<AdminIngestionProfileResponse>, ApiError> {
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
		.admin_ingestion_profile_get(AdminIngestionProfileGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			profile_id,
			version: query.version,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/admin/events/ingestion-profiles/{profile_id}/versions",
	tag = "admin",
	params(("profile_id" = String, Path, description = "Ingestion profile ID.")),
	responses(
		(status = 200, description = "Versions for one ingestion profile.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn admin_ingestion_profile_versions_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(profile_id): Path<String>,
) -> Result<Json<AdminIngestionProfileVersionsListResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.admin_ingestion_profile_versions_list(AdminIngestionProfileVersionsListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			profile_id,
		})
		.await?;

	Ok(Json(response))
}
