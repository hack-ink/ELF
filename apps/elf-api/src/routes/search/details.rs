use crate::routes::{
	ApiError, AppState, ErrorBody, HeaderMap, Json, JsonRejection, Path, RequestContext,
	SearchDetailsBody, SearchDetailsRequest, SearchDetailsResponseV2, State, Uuid,
	search::validation,
};

#[utoipa::path(
	post,
	path = "/v2/searches/{search_id}/notes",
	tag = "search",
	params(("search_id" = Uuid, Path, description = "Search session ID.")),
	request_body = Value,
	responses(
		(status = 200, description = "Hydrated search note details.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Search session was not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(in crate::routes) async fn searches_notes(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(search_id): Path<Uuid>,
	payload: Result<Json<SearchDetailsBody>, JsonRejection>,
) -> Result<Json<SearchDetailsResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(validation::invalid_json_payload)?;

	validation::validate_search_details_payload(&payload)?;

	let response = state
		.service
		.search_details(SearchDetailsRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			search_session_id: search_id,
			payload_level: payload.payload_level.unwrap_or_default(),
			note_ids: payload.note_ids,
			record_hits: payload.record_hits,
		})
		.await?;

	Ok(Json(SearchDetailsResponseV2 {
		search_id: response.search_session_id,
		expires_at: response.expires_at,
		results: response.results,
	}))
}
