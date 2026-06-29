use super::*;

#[utoipa::path(
	post,
	path = "/v2/recall-debug/panel",
	tag = "recall",
	request_body = Value,
	responses(
		(status = 200, description = "Agent-facing cross-layer recall/debug panel.", body = Value),
		(status = 400, description = "Invalid request.", body = ErrorBody),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Scope denied.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn recall_debug_panel(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<RecallDebugPanelBody>, JsonRejection>,
) -> Result<Json<RecallDebugPanelResponse>, ApiError> {
	recall_debug_panel_inner(state, headers, payload, false).await
}

pub(super) async fn admin_recall_debug_panel(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<RecallDebugPanelBody>, JsonRejection>,
) -> Result<Json<RecallDebugPanelResponse>, ApiError> {
	recall_debug_panel_inner(state, headers, payload, true).await
}

pub(super) async fn recall_debug_panel_inner(
	state: AppState,
	headers: HeaderMap,
	payload: Result<Json<RecallDebugPanelBody>, JsonRejection>,
	allow_project_trace_debug: bool,
) -> Result<Json<RecallDebugPanelResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let response = state
		.service
		.recall_debug_panel(RecallDebugPanelRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			trace_id: payload.trace_id,
			query: payload.query,
			docs_query: payload.docs_query,
			knowledge_query: payload.knowledge_query,
			graph_subject: payload.graph_subject,
			graph_predicate: payload.graph_predicate,
			include_dreaming: payload.include_dreaming,
			limit: payload.limit,
			allow_project_trace_debug,
		})
		.await?;

	Ok(Json(response))
}
