use crate::routes::{
	self, ApiError, AppState, ErrorBody, HeaderMap, Json, Path, RequestContext, SpaceGrantItemV2,
	SpaceGrantsListRequest, SpaceGrantsListResponseV2, State,
};

#[utoipa::path(
	get,
	path = "/v2/spaces/{space}/grants",
	tag = "notes",
	params(("space" = String, Path, description = "Shared space name.")),
	responses(
		(status = 200, description = "Space grants.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn space_grants_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(space): Path<String>,
) -> Result<Json<SpaceGrantsListResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let scope = routes::parse_space(space.as_str())?;
	let response = state
		.service
		.space_grants_list(SpaceGrantsListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope,
		})
		.await?;

	Ok(Json(SpaceGrantsListResponseV2 {
		grants: response
			.grants
			.into_iter()
			.map(|item| SpaceGrantItemV2 {
				space: routes::format_space(item.scope).to_string(),
				grantee_kind: item.grantee_kind,
				grantee_agent_id: item.grantee_agent_id,
				granted_by_agent_id: item.granted_by_agent_id,
				granted_at: item.granted_at,
			})
			.collect(),
	}))
}
