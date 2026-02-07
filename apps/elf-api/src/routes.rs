use axum::{
	Json, Router,
	extract::{
		Path, Query, State,
		rejection::{JsonRejection, QueryRejection},
	},
	http::{HeaderMap, StatusCode},
	response::{IntoResponse, Response},
	routing,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;
use elf_service::{
	AddEventRequest, AddEventResponse, AddNoteInput, AddNoteRequest, AddNoteResponse,
	DeleteRequest, DeleteResponse, EventMessage, ListRequest, ListResponse, NoteFetchRequest,
	NoteFetchResponse, RebuildReport, SearchDetailsRequest, SearchDetailsResult,
	SearchExplainRequest, SearchExplainResponse, SearchIndexItem, SearchRequest, SearchResponse,
	SearchSessionGetRequest, SearchTimelineGroup, SearchTimelineRequest, ServiceError,
	TraceGetRequest, TraceGetResponse, UpdateRequest, UpdateResponse,
};

const HEADER_TENANT_ID: &str = "X-ELF-Tenant-Id";
const HEADER_PROJECT_ID: &str = "X-ELF-Project-Id";
const HEADER_AGENT_ID: &str = "X-ELF-Agent-Id";
const HEADER_READ_PROFILE: &str = "X-ELF-Read-Profile";
const MAX_CONTEXT_HEADER_CHARS: usize = 128;

#[derive(Debug, Clone)]
struct RequestContext {
	tenant_id: String,
	project_id: String,
	agent_id: String,
}
impl RequestContext {
	fn from_headers(headers: &HeaderMap) -> Result<Self, ApiError> {
		let tenant_id = required_header(headers, HEADER_TENANT_ID)?;
		let project_id = required_header(headers, HEADER_PROJECT_ID)?;
		let agent_id = required_header(headers, HEADER_AGENT_ID)?;

		Ok(Self { tenant_id, project_id, agent_id })
	}
}

#[derive(Debug, Clone, Deserialize)]
struct NotesIngestRequest {
	scope: String,
	notes: Vec<AddNoteInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct EventsIngestRequest {
	scope: Option<String>,
	dry_run: Option<bool>,
	messages: Vec<EventMessage>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchCreateRequest {
	query: String,
	top_k: Option<u32>,
	candidate_k: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
struct SearchIndexResponseV2 {
	trace_id: Uuid,
	search_id: Uuid,
	#[serde(with = "elf_service::time_serde")]
	expires_at: time::OffsetDateTime,
	items: Vec<SearchIndexItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchSessionGetQuery {
	top_k: Option<u32>,
	touch: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchTimelineQuery {
	group_by: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SearchTimelineResponseV2 {
	search_id: Uuid,
	#[serde(with = "elf_service::time_serde")]
	expires_at: time::OffsetDateTime,
	groups: Vec<SearchTimelineGroup>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchDetailsBody {
	note_ids: Vec<Uuid>,
	record_hits: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct SearchDetailsResponseV2 {
	search_id: Uuid,
	#[serde(with = "elf_service::time_serde")]
	expires_at: time::OffsetDateTime,
	results: Vec<SearchDetailsResult>,
}

#[derive(Debug, Clone, Deserialize)]
struct NotesListQuery {
	scope: Option<String>,
	status: Option<String>,
	#[serde(rename = "type")]
	note_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct NotePatchRequest {
	text: Option<String>,
	importance: Option<f32>,
	confidence: Option<f32>,
	ttl_days: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
	error_code: String,
	message: String,
	fields: Option<Vec<String>>,
}

#[derive(Debug)]
struct ApiError {
	status: StatusCode,
	error_code: String,
	message: String,
	fields: Option<Vec<String>>,
}
impl ApiError {
	fn new(
		status: StatusCode,
		error_code: impl Into<String>,
		message: impl Into<String>,
		fields: Option<Vec<String>>,
	) -> Self {
		Self { status, error_code: error_code.into(), message: message.into(), fields }
	}
}
impl From<ServiceError> for ApiError {
	fn from(err: ServiceError) -> Self {
		match err {
			ServiceError::NonEnglishInput { field } => json_error(
				StatusCode::UNPROCESSABLE_ENTITY,
				"NON_ENGLISH_INPUT",
				"CJK detected; upstream must canonicalize to English before calling ELF.",
				Some(vec![field]),
			),
			ServiceError::InvalidRequest { message } =>
				json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", message, None),
			ServiceError::ScopeDenied { message } =>
				json_error(StatusCode::FORBIDDEN, "SCOPE_DENIED", message, None),
			ServiceError::Provider { message } => {
				tracing::error!(error = %message, "Provider error.");

				json_error(
					StatusCode::INTERNAL_SERVER_ERROR,
					"INTERNAL_ERROR",
					"Internal error.".to_string(),
					None,
				)
			},
			ServiceError::Storage { message } => {
				tracing::error!(error = %message, "Storage error.");

				json_error(
					StatusCode::INTERNAL_SERVER_ERROR,
					"INTERNAL_ERROR",
					"Internal error.".to_string(),
					None,
				)
			},
			ServiceError::Qdrant { message } => {
				tracing::error!(error = %message, "Qdrant error.");

				json_error(
					StatusCode::INTERNAL_SERVER_ERROR,
					"INTERNAL_ERROR",
					"Internal error.".to_string(),
					None,
				)
			},
		}
	}
}
impl IntoResponse for ApiError {
	fn into_response(self) -> Response {
		let body =
			ErrorBody { error_code: self.error_code, message: self.message, fields: self.fields };

		(self.status, Json(body)).into_response()
	}
}

pub fn router(state: AppState) -> Router {
	Router::new()
		.route("/health", routing::get(health))
		.route("/v2/notes/ingest", routing::post(notes_ingest))
		.route("/v2/events/ingest", routing::post(events_ingest))
		.route("/v2/searches", routing::post(searches_create))
		.route("/v2/searches/:search_id", routing::get(searches_get))
		.route("/v2/searches/:search_id/timeline", routing::get(searches_timeline))
		.route("/v2/searches/:search_id/notes", routing::post(searches_notes))
		.route("/v2/notes", routing::get(notes_list))
		.route(
			"/v2/notes/:note_id",
			routing::get(notes_get).patch(notes_patch).delete(notes_delete),
		)
		.with_state(state)
}

pub fn admin_router(state: AppState) -> Router {
	Router::new()
		.route("/v2/admin/qdrant/rebuild", routing::post(rebuild_qdrant))
		.route("/v2/admin/searches/raw", routing::post(searches_raw))
		.route("/v2/admin/traces/:trace_id", routing::get(trace_get))
		.route("/v2/admin/trace-items/:item_id", routing::get(trace_item_get))
		.with_state(state)
}

fn json_error(
	status: StatusCode,
	code: &str,
	message: impl Into<String>,
	fields: Option<Vec<String>>,
) -> ApiError {
	ApiError::new(status, code, message, fields)
}

fn required_header(headers: &HeaderMap, name: &'static str) -> Result<String, ApiError> {
	let raw = headers.get(name).ok_or_else(|| {
		json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			format!("{name} header is required."),
			Some(vec![format!("$.headers.{name}")]),
		)
	})?;
	let value = raw.to_str().map_err(|_| {
		json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			format!("{name} header must be a valid string."),
			Some(vec![format!("$.headers.{name}")]),
		)
	})?;
	let trimmed = value.trim();

	if trimmed.is_empty() {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			format!("{name} header must be non-empty."),
			Some(vec![format!("$.headers.{name}")]),
		));
	}
	if trimmed.chars().count() > MAX_CONTEXT_HEADER_CHARS {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			format!("{name} header is too long."),
			Some(vec![format!("$.headers.{name}")]),
		));
	}
	if elf_domain::cjk::contains_cjk(trimmed) {
		return Err(json_error(
			StatusCode::UNPROCESSABLE_ENTITY,
			"NON_ENGLISH_INPUT",
			"CJK detected; upstream must canonicalize to English before calling ELF.".to_string(),
			Some(vec![format!("$.headers.{name}")]),
		));
	}

	Ok(trimmed.to_string())
}

fn required_read_profile(headers: &HeaderMap) -> Result<String, ApiError> {
	required_header(headers, HEADER_READ_PROFILE)
}

async fn health() -> StatusCode {
	StatusCode::OK
}

async fn notes_ingest(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<NotesIngestRequest>, JsonRejection>,
) -> Result<Json<AddNoteResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let response = state
		.service
		.add_note(AddNoteRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope: payload.scope,
			notes: payload.notes,
		})
		.await?;

	Ok(Json(response))
}

async fn events_ingest(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<EventsIngestRequest>, JsonRejection>,
) -> Result<Json<AddEventResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let response = state
		.service
		.add_event(AddEventRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope: payload.scope,
			dry_run: payload.dry_run,
			messages: payload.messages,
		})
		.await?;

	Ok(Json(response))
}

async fn searches_create(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<SearchCreateRequest>, JsonRejection>,
) -> Result<Json<SearchIndexResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let response = state
		.service
		.search(SearchRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			query: payload.query,
			top_k: payload.top_k,
			candidate_k: payload.candidate_k,
			record_hits: Some(false),
		})
		.await?;

	Ok(Json(SearchIndexResponseV2 {
		trace_id: response.trace_id,
		search_id: response.search_session_id,
		expires_at: response.expires_at,
		items: response.items,
	}))
}

async fn searches_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(search_id): Path<Uuid>,
	query: Result<Query<SearchSessionGetQuery>, QueryRejection>,
) -> Result<Json<SearchIndexResponseV2>, ApiError> {
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
		.search_session_get(SearchSessionGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			search_session_id: search_id,
			top_k: query.top_k,
			touch: query.touch,
		})
		.await?;

	Ok(Json(SearchIndexResponseV2 {
		trace_id: response.trace_id,
		search_id: response.search_session_id,
		expires_at: response.expires_at,
		items: response.items,
	}))
}

async fn searches_timeline(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(search_id): Path<Uuid>,
	query: Result<Query<SearchTimelineQuery>, QueryRejection>,
) -> Result<Json<SearchTimelineResponseV2>, ApiError> {
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
		.search_timeline(SearchTimelineRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			search_session_id: search_id,
			group_by: query.group_by,
		})
		.await?;

	Ok(Json(SearchTimelineResponseV2 {
		search_id: response.search_session_id,
		expires_at: response.expires_at,
		groups: response.groups,
	}))
}

async fn searches_notes(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(search_id): Path<Uuid>,
	payload: Result<Json<SearchDetailsBody>, JsonRejection>,
) -> Result<Json<SearchDetailsResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let response = state
		.service
		.search_details(SearchDetailsRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			search_session_id: search_id,
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

async fn notes_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	query: Result<Query<NotesListQuery>, QueryRejection>,
) -> Result<Json<ListResponse>, ApiError> {
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
		.list(ListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: Some(ctx.agent_id),
			scope: query.scope,
			status: query.status,
			note_type: query.note_type,
		})
		.await?;

	Ok(Json(response))
}

async fn notes_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(note_id): Path<Uuid>,
) -> Result<Json<NoteFetchResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.get_note(NoteFetchRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			note_id,
		})
		.await?;

	Ok(Json(response))
}

async fn notes_patch(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(note_id): Path<Uuid>,
	payload: Result<Json<NotePatchRequest>, JsonRejection>,
) -> Result<Json<UpdateResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let response = state
		.service
		.update(UpdateRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			note_id,
			text: payload.text,
			importance: payload.importance,
			confidence: payload.confidence,
			ttl_days: payload.ttl_days,
		})
		.await?;

	Ok(Json(response))
}

async fn notes_delete(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(note_id): Path<Uuid>,
) -> Result<Json<DeleteResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.delete(DeleteRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			note_id,
		})
		.await?;

	Ok(Json(response))
}

async fn rebuild_qdrant(State(state): State<AppState>) -> Result<Json<RebuildReport>, ApiError> {
	let response = state.service.rebuild_qdrant().await?;

	Ok(Json(response))
}

async fn searches_raw(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<SearchCreateRequest>, JsonRejection>,
) -> Result<Json<SearchResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let response = state
		.service
		.search_raw(SearchRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			query: payload.query,
			top_k: payload.top_k,
			candidate_k: payload.candidate_k,
			record_hits: Some(false),
		})
		.await?;

	Ok(Json(response))
}

async fn trace_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(trace_id): Path<Uuid>,
) -> Result<Json<TraceGetResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.trace_get(TraceGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			trace_id,
		})
		.await?;

	Ok(Json(response))
}

async fn trace_item_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(item_id): Path<Uuid>,
) -> Result<Json<SearchExplainResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.search_explain(SearchExplainRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			result_handle: item_id,
		})
		.await?;

	Ok(Json(response))
}
