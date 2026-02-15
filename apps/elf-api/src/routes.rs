use axum::{
	Json, Router,
	body::Body,
	extract::{
		DefaultBodyLimit, Path, Query, State,
		rejection::{JsonRejection, QueryRejection},
	},
	http::{HeaderMap, Request, StatusCode},
	middleware::{self, Next},
	response::{IntoResponse, Response},
	routing,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::state::AppState;
use elf_service::{
	AddEventRequest, AddEventResponse, AddNoteInput, AddNoteRequest, AddNoteResponse,
	DeleteRequest, DeleteResponse, Error, EventMessage, ListRequest, ListResponse,
	NoteFetchRequest, NoteFetchResponse, RankingRequestOverride, RebuildReport,
	SearchDetailsRequest, SearchDetailsResult, SearchExplainRequest, SearchExplainResponse,
	SearchIndexItem, SearchRequest, SearchResponse, SearchSessionGetRequest, SearchTimelineGroup,
	SearchTimelineRequest, TraceGetRequest, TraceGetResponse, UpdateRequest, UpdateResponse,
};

const HEADER_TENANT_ID: &str = "X-ELF-Tenant-Id";
const HEADER_PROJECT_ID: &str = "X-ELF-Project-Id";
const HEADER_AGENT_ID: &str = "X-ELF-Agent-Id";
const HEADER_READ_PROFILE: &str = "X-ELF-Read-Profile";
const HEADER_AUTH_TOKEN: &str = "X-ELF-Auth-Token";
const HEADER_AUTHORIZATION: &str = "Authorization";
const MAX_CONTEXT_HEADER_CHARS: usize = 128;
const MAX_REQUEST_BYTES: usize = 1_048_576;
const MAX_NOTES_PER_INGEST: usize = 256;
const MAX_MESSAGES_PER_EVENT: usize = 256;
const MAX_MESSAGE_CHARS: usize = 16_384;
const MAX_QUERY_CHARS: usize = 2_048;
const MAX_NOTE_IDS_PER_DETAILS: usize = 256;
const MAX_TOP_K: u32 = 100;
const MAX_CANDIDATE_K: u32 = 1_000;
const MAX_ERROR_LOG_CHARS: usize = 1_024;

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug, Deserialize)]
struct NotesIngestRequest {
	scope: String,
	notes: Vec<AddNoteInput>,
}

#[derive(Clone, Debug, Deserialize)]
struct EventsIngestRequest {
	scope: Option<String>,
	dry_run: Option<bool>,
	messages: Vec<EventMessage>,
}

#[derive(Clone, Debug, Deserialize)]
struct SearchCreateRequest {
	query: String,
	top_k: Option<u32>,
	candidate_k: Option<u32>,
	ranking: Option<RankingRequestOverride>,
}

#[derive(Clone, Debug, Serialize)]
struct SearchIndexResponseV2 {
	trace_id: Uuid,
	search_id: Uuid,
	#[serde(with = "elf_service::time_serde")]
	expires_at: OffsetDateTime,
	items: Vec<SearchIndexItem>,
}

#[derive(Clone, Debug, Deserialize)]
struct SearchSessionGetQuery {
	top_k: Option<u32>,
	touch: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
struct SearchTimelineQuery {
	group_by: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct SearchTimelineResponseV2 {
	search_id: Uuid,
	#[serde(with = "elf_service::time_serde")]
	expires_at: OffsetDateTime,
	groups: Vec<SearchTimelineGroup>,
}

#[derive(Clone, Debug, Deserialize)]
struct SearchDetailsBody {
	note_ids: Vec<Uuid>,
	record_hits: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
struct SearchDetailsResponseV2 {
	search_id: Uuid,
	#[serde(with = "elf_service::time_serde")]
	expires_at: OffsetDateTime,
	results: Vec<SearchDetailsResult>,
}

#[derive(Clone, Debug, Deserialize)]
struct NotesListQuery {
	scope: Option<String>,
	status: Option<String>,
	r#type: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
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
impl From<Error> for ApiError {
	fn from(err: Error) -> Self {
		match err {
			Error::NonEnglishInput { field } => json_error(
				StatusCode::UNPROCESSABLE_ENTITY,
				"NON_ENGLISH_INPUT",
				"CJK detected; upstream must canonicalize to English before calling ELF.",
				Some(vec![field]),
			),
			Error::InvalidRequest { message } =>
				json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", message, None),
			Error::ScopeDenied { message } =>
				json_error(StatusCode::FORBIDDEN, "SCOPE_DENIED", message, None),
			Error::Provider { message } => {
				let sanitized = sanitize_log_text(message.as_str());

				tracing::error!(error = %sanitized, "Provider error.");

				json_error(
					StatusCode::INTERNAL_SERVER_ERROR,
					"INTERNAL_ERROR",
					"Internal error.".to_string(),
					None,
				)
			},
			Error::Storage { message } => {
				let sanitized = sanitize_log_text(message.as_str());

				tracing::error!(error = %sanitized, "Storage error.");

				json_error(
					StatusCode::INTERNAL_SERVER_ERROR,
					"INTERNAL_ERROR",
					"Internal error.".to_string(),
					None,
				)
			},
			Error::Qdrant { message } => {
				let sanitized = sanitize_log_text(message.as_str());

				tracing::error!(error = %sanitized, "Qdrant error.");

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
	let auth_state = state.clone();

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
		.layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
		.layer(middleware::from_fn_with_state(auth_state, api_auth_middleware))
}

pub fn admin_router(state: AppState) -> Router {
	let auth_state = state.clone();

	Router::new()
		.route("/v2/admin/qdrant/rebuild", routing::post(rebuild_qdrant))
		.route("/v2/admin/searches/raw", routing::post(searches_raw))
		.route("/v2/admin/traces/:trace_id", routing::get(trace_get))
		.route("/v2/admin/trace-items/:item_id", routing::get(trace_item_get))
		.with_state(state)
		.layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
		.layer(middleware::from_fn_with_state(auth_state, admin_auth_middleware))
}

fn json_error(
	status: StatusCode,
	code: &str,
	message: impl Into<String>,
	fields: Option<Vec<String>>,
) -> ApiError {
	ApiError::new(status, code, message, fields)
}

fn sanitize_log_text(text: &str) -> String {
	let mut parts = Vec::new();
	let mut redact_next = false;

	for raw in text.split_whitespace() {
		let mut word = raw.to_string();

		if redact_next {
			word = "[REDACTED]".to_string();
			redact_next = false;
		}
		if raw.eq_ignore_ascii_case("bearer") {
			redact_next = true;
		}

		let lowered = raw.to_ascii_lowercase();

		for key in ["api_key", "apikey", "password", "secret", "token"] {
			if lowered.contains(key) && (lowered.contains('=') || lowered.contains(':')) {
				let sep = if raw.contains('=') { '=' } else { ':' };
				let prefix = match raw.split(sep).next() {
					Some(prefix) => prefix,
					None => raw,
				};

				word = format!("{prefix}{sep}[REDACTED]");

				break;
			}
		}

		parts.push(word);
	}

	let mut out = parts.join(" ");

	if out.chars().count() > MAX_ERROR_LOG_CHARS {
		out = out.chars().take(MAX_ERROR_LOG_CHARS).collect();

		out.push_str("...");
	}

	out
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

fn is_authorized(headers: &HeaderMap, expected: Option<&str>) -> bool {
	let Some(expected) = expected else { return true };

	if let Some(raw) = headers.get(HEADER_AUTH_TOKEN)
		&& let Ok(value) = raw.to_str()
		&& value.trim() == expected
	{
		return true;
	}
	if let Some(raw) = headers.get(HEADER_AUTHORIZATION)
		&& let Ok(value) = raw.to_str()
	{
		let value = value.trim();

		if let Some(token) = value.strip_prefix("Bearer ").or_else(|| value.strip_prefix("bearer "))
		{
			return token.trim() == expected;
		}
	}

	false
}

fn configured_token(raw: &str) -> Option<&str> {
	let token = raw.trim();

	if token.is_empty() { None } else { Some(token) }
}

async fn api_auth_middleware(
	State(state): State<AppState>,
	req: Request<Body>,
	next: Next,
) -> Response {
	let expected = configured_token(&state.service.cfg.security.api_auth_token);

	if expected.is_some() && !is_authorized(req.headers(), expected) {
		return json_error(
			StatusCode::UNAUTHORIZED,
			"UNAUTHORIZED",
			"Authentication required.",
			None,
		)
		.into_response();
	}

	next.run(req).await
}

async fn admin_auth_middleware(
	State(state): State<AppState>,
	req: Request<Body>,
	next: Next,
) -> Response {
	let expected = configured_token(&state.service.cfg.security.admin_auth_token)
		.or_else(|| configured_token(&state.service.cfg.security.api_auth_token));

	if expected.is_some() && !is_authorized(req.headers(), expected) {
		return json_error(
			StatusCode::UNAUTHORIZED,
			"UNAUTHORIZED",
			"Authentication required.",
			None,
		)
		.into_response();
	}

	next.run(req).await
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

	if payload.notes.len() > MAX_NOTES_PER_INGEST {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Notes list is too large.",
			Some(vec!["$.notes".to_string()]),
		));
	}

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

	if payload.messages.len() > MAX_MESSAGES_PER_EVENT {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Messages list is too large.",
			Some(vec!["$.messages".to_string()]),
		));
	}

	for (idx, msg) in payload.messages.iter().enumerate() {
		if msg.content.chars().count() > MAX_MESSAGE_CHARS {
			return Err(json_error(
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

	if payload.query.chars().count() > MAX_QUERY_CHARS {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Query is too long.",
			Some(vec!["$.query".to_string()]),
		));
	}
	if payload.top_k.unwrap_or(state.service.cfg.memory.top_k) > MAX_TOP_K {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"top_k is too large.",
			Some(vec!["$.top_k".to_string()]),
		));
	}
	if payload.candidate_k.unwrap_or(state.service.cfg.memory.candidate_k) > MAX_CANDIDATE_K {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"candidate_k is too large.",
			Some(vec!["$.candidate_k".to_string()]),
		));
	}
	if payload.ranking.is_some() {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Ranking overrides are only supported on admin endpoints.".to_string(),
			None,
		));
	}

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
			ranking: None,
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

	if payload.note_ids.len() > MAX_NOTE_IDS_PER_DETAILS {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"note_ids list is too large.",
			Some(vec!["$.note_ids".to_string()]),
		));
	}

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
			r#type: query.r#type,
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

	if payload.query.chars().count() > MAX_QUERY_CHARS {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Query is too long.",
			Some(vec!["$.query".to_string()]),
		));
	}
	if payload.top_k.unwrap_or(state.service.cfg.memory.top_k) > MAX_TOP_K {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"top_k is too large.",
			Some(vec!["$.top_k".to_string()]),
		));
	}
	if payload.candidate_k.unwrap_or(state.service.cfg.memory.candidate_k) > MAX_CANDIDATE_K {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"candidate_k is too large.",
			Some(vec!["$.candidate_k".to_string()]),
		));
	}

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
			ranking: payload.ranking,
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
