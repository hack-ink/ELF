use crate::routes::{
	self, AddEventRequest, AddEventResponse, ApiError, AppState, ErrorBody, EventsIngestRequest,
	Extension, HeaderMap, Json, JsonRejection, MAX_MESSAGE_CHARS, MAX_MESSAGES_PER_EVENT,
	RequestContext, SecurityAuthRole, State, StatusCode,
};

#[utoipa::path(
	post,
	path = "/v2/events/ingest",
	tag = "events",
	request_body = Value,
	responses(
		(status = 200, description = "Event messages were processed.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn events_ingest(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	payload: Result<Json<EventsIngestRequest>, JsonRejection>,
) -> Result<Json<AddEventResponse>, ApiError> {
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
	let role = role.map(|Extension(role)| role);

	if payload.scope.as_deref().map(str::trim) == Some("org_shared") {
		routes::require_admin_for_org_shared_writes(
			state.service.cfg.security.auth_mode.as_str(),
			role,
		)?;
	}
	if payload.messages.len() > MAX_MESSAGES_PER_EVENT {
		return Err(routes::json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Messages list is too large.",
			Some(vec!["$.messages".to_string()]),
		));
	}

	for (idx, msg) in payload.messages.iter().enumerate() {
		if msg.content.chars().count() > MAX_MESSAGE_CHARS {
			return Err(routes::json_error(
				StatusCode::BAD_REQUEST,
				"INVALID_REQUEST",
				"Message content is too long.",
				Some(vec![format!("$.messages[{idx}].content")]),
			));
		}
	}

	let response = state
		.service
		.add_event(AddEventRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope: payload.scope,
			dry_run: payload.dry_run,
			ingestion_profile: payload.ingestion_profile,
			messages: payload.messages,
		})
		.await?;

	Ok(Json(response))
}
