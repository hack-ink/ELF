use super::*;

#[utoipa::path(
	post,
	path = "/v2/work-journal/entries",
	tag = "work_journal",
	request_body = Value,
	responses(
		(status = 200, description = "Work Journal entry was stored.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn work_journal_entry_create(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	payload: Result<Json<WorkJournalEntryCreateBody>, JsonRejection>,
) -> Result<Json<WorkJournalEntryCreateResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let role = role.map(|Extension(role)| role);

	if payload.scope.trim() == "org_shared" {
		require_admin_for_org_shared_writes(state.service.cfg.security.auth_mode.as_str(), role)?;
	}

	let response = state
		.service
		.work_journal_entry_create(WorkJournalEntryCreateRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			entry_id: payload.entry_id,
			scope: payload.scope,
			session_id: payload.session_id,
			family: payload.family,
			title: payload.title,
			body: payload.body,
			source_refs: payload.source_refs,
			write_policy: payload.write_policy,
			explicit_next_steps: payload.explicit_next_steps,
			inferred_next_steps: payload.inferred_next_steps,
			rejected_options: payload.rejected_options,
			promotion_boundary: payload.promotion_boundary,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v2/work-journal/entries/{entry_id}",
	tag = "work_journal",
	responses(
		(status = 200, description = "Work Journal entry metadata.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 404, description = "Work Journal entry not found.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn work_journal_entry_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(entry_id): Path<Uuid>,
) -> Result<Json<WorkJournalEntryResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let response = state
		.service
		.work_journal_entry_get(WorkJournalEntryGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			entry_id,
		})
		.await?;

	Ok(Json(response))
}

#[utoipa::path(
	post,
	path = "/v2/work-journal/readback",
	tag = "work_journal",
	request_body = Value,
	responses(
		(status = 200, description = "Work Journal session readback.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 422, description = "Non-English input rejected.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn work_journal_session_readback(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<WorkJournalSessionReadbackBody>, JsonRejection>,
) -> Result<Json<WorkJournalSessionReadbackResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let response = state
		.service
		.work_journal_session_readback(WorkJournalSessionReadbackRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			session_id: payload.session_id,
			families: payload.families,
			limit: payload.limit,
		})
		.await?;

	Ok(Json(response))
}
