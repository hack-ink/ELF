use axum::{
	Json, Router,
	body::Body,
	extract::{
		DefaultBodyLimit, Extension, Path, Query, State,
		rejection::{JsonRejection, QueryRejection},
	},
	http::{HeaderMap, Request, StatusCode},
	middleware::{self, Next},
	response::{IntoResponse, Response},
	routing,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::state::AppState;
use elf_config::{SecurityAuthKey, SecurityAuthRole};
use elf_service::{
	AddEventRequest, AddEventResponse, AddNoteInput, AddNoteRequest, AddNoteResponse,
	AdminGraphPredicateAliasAddRequest, AdminGraphPredicateAliasesListRequest,
	AdminGraphPredicateAliasesResponse, AdminGraphPredicatePatchRequest,
	AdminGraphPredicateResponse, AdminGraphPredicatesListRequest, AdminGraphPredicatesListResponse,
	AdminIngestionProfileCreateRequest, AdminIngestionProfileDefaultGetRequest,
	AdminIngestionProfileDefaultResponse, AdminIngestionProfileDefaultSetRequest,
	AdminIngestionProfileGetRequest, AdminIngestionProfileListRequest,
	AdminIngestionProfileResponse, AdminIngestionProfileVersionsListRequest,
	AdminIngestionProfileVersionsListResponse, AdminIngestionProfilesListResponse, DeleteRequest,
	DeleteResponse, DocsExcerptResponse, DocsExcerptsGetRequest, DocsGetRequest, DocsGetResponse,
	DocsPutRequest, DocsPutResponse, DocsSearchL0Request, DocsSearchL0Response, Error,
	EventMessage, GranteeKind, IngestionProfileSelector, ListRequest, ListResponse,
	NoteFetchRequest, NoteFetchResponse, PayloadLevel, PublishNoteRequest, QueryPlan,
	RankingRequestOverride, RebuildReport, SearchDetailsRequest, SearchDetailsResult,
	SearchExplainRequest, SearchExplainResponse, SearchIndexItem, SearchRequest, SearchResponse,
	SearchSessionGetRequest, SearchTimelineGroup, SearchTimelineRequest, SearchTrajectoryResponse,
	ShareScope, SpaceGrantRevokeRequest, SpaceGrantRevokeResponse, SpaceGrantUpsertRequest,
	SpaceGrantsListRequest, TextPositionSelector, TextQuoteSelector, TraceBundleGetRequest,
	TraceBundleResponse, TraceGetRequest, TraceGetResponse, TraceRecentListRequest,
	TraceRecentListResponse, TraceTrajectoryGetRequest, UnpublishNoteRequest, UpdateRequest,
	UpdateResponse, search::TraceBundleMode,
};

const HEADER_TENANT_ID: &str = "X-ELF-Tenant-Id";
const HEADER_PROJECT_ID: &str = "X-ELF-Project-Id";
const HEADER_AGENT_ID: &str = "X-ELF-Agent-Id";
const HEADER_READ_PROFILE: &str = "X-ELF-Read-Profile";
const HEADER_AUTHORIZATION: &str = "Authorization";
const HEADER_TRUSTED_TOKEN_ID: &str = "X-ELF-Trusted-Token-Id";
const MAX_CONTEXT_HEADER_CHARS: usize = 128;
const MAX_REQUEST_BYTES: usize = 1_048_576;
const MAX_DOC_REQUEST_BYTES: usize = 4 * 1_024 * 1_024;
const MAX_NOTES_PER_INGEST: usize = 256;
const MAX_MESSAGES_PER_EVENT: usize = 256;
const MAX_MESSAGE_CHARS: usize = 16_384;
const MAX_QUERY_CHARS: usize = 2_048;
const DOC_STATUSES: [&str; 2] = ["active", "deleted"];
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
	ingestion_profile: Option<IngestionProfileSelector>,
	messages: Vec<EventMessage>,
}

#[derive(Clone, Debug, Deserialize)]
struct DocsPutBody {
	scope: String,
	doc_type: Option<String>,
	title: Option<String>,
	#[serde(default)]
	source_ref: Value,
	content: String,
}

#[derive(Clone, Debug, Deserialize)]
struct DocsSearchL0Body {
	query: String,
	scope: Option<String>,
	status: Option<String>,
	doc_type: Option<String>,
	agent_id: Option<String>,
	thread_id: Option<String>,
	updated_after: Option<String>,
	updated_before: Option<String>,
	ts_gte: Option<String>,
	ts_lte: Option<String>,
	top_k: Option<u32>,
	candidate_k: Option<u32>,
	explain: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
struct DocsExcerptsGetBody {
	doc_id: Uuid,
	level: String,
	chunk_id: Option<Uuid>,
	quote: Option<TextQuoteSelector>,
	position: Option<TextPositionSelector>,
	explain: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
struct SearchCreateRequest {
	query: String,
	top_k: Option<u32>,
	candidate_k: Option<u32>,

	filter: Option<Value>,
	payload_level: Option<PayloadLevel>,
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

#[derive(Clone, Debug, Serialize)]
struct SearchIndexPlannedResponseV2 {
	trace_id: Uuid,
	search_id: Uuid,
	#[serde(with = "elf_service::time_serde")]
	expires_at: OffsetDateTime,
	items: Vec<SearchIndexItem>,
	query_plan: QueryPlan,
}

#[derive(Clone, Debug, Deserialize)]
struct SearchSessionGetQuery {
	payload_level: Option<PayloadLevel>,
	top_k: Option<u32>,
	touch: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
struct SearchTimelineQuery {
	payload_level: Option<PayloadLevel>,
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
	payload_level: Option<PayloadLevel>,
	record_hits: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
struct AdminIngestionProfileCreateBody {
	profile_id: String,
	version: Option<i32>,
	profile: Value,
	created_by: String,
}

#[derive(Clone, Debug, Deserialize)]
struct AdminIngestionProfileGetQuery {
	version: Option<i32>,
}

#[derive(Clone, Debug, Deserialize)]
struct AdminIngestionProfileDefaultSetBody {
	profile_id: String,
	version: Option<i32>,
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

#[derive(Clone, Debug, Deserialize)]
struct AdminGraphPredicatesListQuery {
	scope: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct AdminGraphPredicatePatchBody {
	status: Option<String>,
	cardinality: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct AdminGraphPredicateAliasAddBody {
	alias: String,
}

#[derive(Clone, Debug, Deserialize)]
struct TraceRecentListQuery {
	limit: Option<u32>,
	cursor_created_at: Option<String>,
	cursor_trace_id: Option<Uuid>,
	agent_id: Option<String>,
	read_profile: Option<String>,
	created_after: Option<String>,
	created_before: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct TraceBundleGetQuery {
	mode: Option<TraceBundleMode>,
	stage_items_limit: Option<u32>,
	candidates_limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
struct ShareScopeBody {
	space: String,
}

#[derive(Clone, Debug, Deserialize)]
struct SpaceGrantUpsertBody {
	grantee_kind: GranteeKind,
	grantee_agent_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct PublishResponseV2 {
	note_id: Uuid,
	space: String,
}

#[derive(Clone, Debug, Serialize)]
struct SpaceGrantUpsertResponseV2 {
	space: String,
	grantee_kind: GranteeKind,
	grantee_agent_id: Option<String>,
	granted: bool,
}

#[derive(Clone, Debug, Serialize)]
struct SpaceGrantItemV2 {
	space: String,
	grantee_kind: GranteeKind,
	grantee_agent_id: Option<String>,
	granted_by_agent_id: String,
	granted_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize)]
struct SpaceGrantsListResponseV2 {
	grants: Vec<SpaceGrantItemV2>,
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
				"Non-English input detected; upstream must canonicalize to English before calling ELF.",
				Some(vec![field]),
			),
			Error::InvalidRequest { message } =>
				json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", message, None),
			Error::ScopeDenied { message } =>
				json_error(StatusCode::FORBIDDEN, "SCOPE_DENIED", message, None),
			Error::NotFound { message } =>
				json_error(StatusCode::NOT_FOUND, "NOT_FOUND", message, None),
			Error::Conflict { message } =>
				json_error(StatusCode::CONFLICT, "CONFLICT", message, None),
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
	let api_router = Router::new()
		.route("/health", routing::get(health))
		.route("/v2/notes/ingest", routing::post(notes_ingest))
		.route("/v2/events/ingest", routing::post(events_ingest))
		.route("/v2/search/quick", routing::post(search_quick_create))
		.route("/v2/search/planned", routing::post(search_planned_create))
		.route("/v2/searches/:search_id", routing::get(searches_get))
		.route("/v2/searches/:search_id/timeline", routing::get(searches_timeline))
		.route("/v2/searches/:search_id/notes", routing::post(searches_notes))
		.route("/v2/notes", routing::get(notes_list))
		.route(
			"/v2/notes/:note_id",
			routing::get(notes_get).patch(notes_patch).delete(notes_delete),
		)
		.route("/v2/notes/:note_id/publish", routing::post(notes_publish))
		.route("/v2/notes/:note_id/unpublish", routing::post(notes_unpublish))
		.route("/v2/spaces/:space/grants", routing::get(space_grants_list).post(space_grant_upsert))
		.route("/v2/spaces/:space/grants/revoke", routing::post(space_grant_revoke))
		.with_state(state.clone())
		.layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES));
	let docs_router = Router::new()
		.route("/v2/docs", routing::post(docs_put))
		.route("/v2/docs/:doc_id", routing::get(docs_get))
		.route("/v2/docs/search/l0", routing::post(docs_search_l0))
		.route("/v2/docs/excerpts", routing::post(docs_excerpts_get))
		.with_state(state)
		.layer(DefaultBodyLimit::max(MAX_DOC_REQUEST_BYTES));

	Router::new()
		.merge(api_router)
		.merge(docs_router)
		.layer(middleware::from_fn_with_state(auth_state, api_auth_middleware))
}

pub fn admin_router(state: AppState) -> Router {
	let auth_state = state.clone();

	Router::new()
		.route(
			"/v2/admin/events/ingestion-profiles/default",
			routing::get(admin_ingestion_profile_default_get)
				.put(admin_ingestion_profile_default_set),
		)
		.route(
			"/v2/admin/events/ingestion-profiles/:profile_id/versions",
			routing::get(admin_ingestion_profile_versions_list),
		)
		.route(
			"/v2/admin/events/ingestion-profiles/:profile_id",
			routing::get(admin_ingestion_profile_get),
		)
		.route(
			"/v2/admin/events/ingestion-profiles",
			routing::get(admin_ingestion_profiles_list).post(admin_ingestion_profile_create),
		)
		.route("/v2/admin/qdrant/rebuild", routing::post(rebuild_qdrant))
		.route("/v2/admin/searches/raw", routing::post(searches_raw))
		.route("/v2/admin/traces/recent", routing::get(trace_recent_list))
		.route("/v2/admin/traces/:trace_id", routing::get(trace_get))
		.route("/v2/admin/traces/:trace_id/bundle", routing::get(trace_bundle_get))
		.route("/v2/admin/trajectories/:trace_id", routing::get(trace_trajectory_get))
		.route("/v2/admin/trace-items/:item_id", routing::get(trace_item_get))
		.route("/v2/admin/graph/predicates", routing::get(admin_graph_predicates_list))
		.route(
			"/v2/admin/graph/predicates/:predicate_id",
			routing::patch(admin_graph_predicate_patch),
		)
		.route(
			"/v2/admin/graph/predicates/:predicate_id/aliases",
			routing::post(admin_graph_predicate_alias_add).get(admin_graph_predicate_aliases_list),
		)
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
	if !elf_domain::english_gate::is_english_identifier(trimmed) {
		return Err(json_error(
			StatusCode::UNPROCESSABLE_ENTITY,
			"NON_ENGLISH_INPUT",
			"Non-English input detected; upstream must canonicalize to English before calling ELF."
				.to_string(),
			Some(vec![format!("$.headers.{name}")]),
		));
	}

	Ok(trimmed.to_string())
}

fn required_read_profile(headers: &HeaderMap) -> Result<String, ApiError> {
	required_header(headers, HEADER_READ_PROFILE)
}

fn parse_space(scope: &str) -> Result<ShareScope, ApiError> {
	match scope {
		"team_shared" | "project_shared" => Ok(ShareScope::ProjectShared),
		"org_shared" => Ok(ShareScope::OrgShared),
		_ => Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Invalid space.".to_string(),
			Some(vec!["$.space".to_string()]),
		)),
	}
}

fn format_space(scope: ShareScope) -> &'static str {
	match scope {
		ShareScope::ProjectShared => "team_shared",
		ShareScope::OrgShared => "org_shared",
	}
}

fn format_scope(scope: &str) -> Result<&'static str, ApiError> {
	match scope {
		"project_shared" => Ok("team_shared"),
		"org_shared" => Ok("org_shared"),
		"agent_private" => Ok("agent_private"),
		_ => Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Invalid space.".to_string(),
			Some(vec!["$.space".to_string()]),
		)),
	}
}

fn trusted_token_id(headers: &HeaderMap) -> Option<String> {
	let raw = headers.get(HEADER_TRUSTED_TOKEN_ID)?;
	let value = raw.to_str().ok()?.trim();

	if value.is_empty() { None } else { Some(value.to_string()) }
}

fn sanitize_trusted_token_header(headers: &mut HeaderMap) {
	headers.remove(HEADER_TRUSTED_TOKEN_ID);
}

fn effective_token_id(auth_mode: &str, headers: &HeaderMap) -> Option<String> {
	match auth_mode.trim() {
		"static_keys" => trusted_token_id(headers),
		_ => None,
	}
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
	let raw = headers.get(HEADER_AUTHORIZATION)?;
	let value = raw.to_str().ok()?.trim();
	let token = value.strip_prefix("Bearer ")?;
	let token = token.trim();

	if token.is_empty() { None } else { Some(token.to_string()) }
}

fn resolve_auth_key<'a>(
	headers: &HeaderMap,
	auth_keys: &'a [SecurityAuthKey],
) -> Result<&'a SecurityAuthKey, ApiError> {
	let token = bearer_token(headers).ok_or_else(|| {
		json_error(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "Authentication required.", None)
	})?;

	auth_keys.iter().find(|key| key.token == token).ok_or_else(|| {
		json_error(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "Authentication required.", None)
	})
}

fn set_context_header(
	headers: &mut HeaderMap,
	name: &'static str,
	value: &str,
) -> Result<(), ApiError> {
	let header_value = value.parse().map_err(|_| {
		json_error(
			StatusCode::INTERNAL_SERVER_ERROR,
			"INTERNAL_ERROR",
			format!("Invalid configured auth context for {name}."),
			None,
		)
	})?;

	headers.insert(name, header_value);

	Ok(())
}

fn apply_auth_key_context(headers: &mut HeaderMap, key: &SecurityAuthKey) -> Result<(), ApiError> {
	let agent_id = key.agent_id.as_deref().ok_or_else(|| {
		json_error(StatusCode::FORBIDDEN, "FORBIDDEN", "Token is not scoped to an agent_id.", None)
	})?;

	set_context_header(headers, HEADER_TENANT_ID, key.tenant_id.as_str())?;
	set_context_header(headers, HEADER_PROJECT_ID, key.project_id.as_str())?;
	set_context_header(headers, HEADER_AGENT_ID, agent_id)?;
	set_context_header(headers, HEADER_READ_PROFILE, key.read_profile.as_str())?;
	set_context_header(headers, HEADER_TRUSTED_TOKEN_ID, key.token_id.as_str())?;

	Ok(())
}

fn require_admin_for_org_shared_writes(
	auth_mode: &str,
	role: Option<SecurityAuthRole>,
) -> Result<(), ApiError> {
	if auth_mode.trim() != "static_keys" {
		return Ok(());
	}
	if matches!(role, Some(SecurityAuthRole::Admin | SecurityAuthRole::SuperAdmin)) {
		return Ok(());
	}

	Err(json_error(StatusCode::FORBIDDEN, "FORBIDDEN", "Admin token required.", None))
}

fn parse_optional_rfc3339(
	raw: Option<&String>,
	path: &str,
) -> Result<Option<OffsetDateTime>, ApiError> {
	let Some(raw) = raw else {
		return Ok(None);
	};
	let raw = raw.trim();

	if raw.is_empty() {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			format!("{path} must be non-empty."),
			Some(vec![path.to_string()]),
		));
	}

	OffsetDateTime::parse(raw, &Rfc3339).map(Some).map_err(|_| {
		json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			format!("{path} must be an RFC3339 datetime string."),
			Some(vec![path.to_string()]),
		)
	})
}

async fn api_auth_middleware(
	State(state): State<AppState>,
	req: Request<Body>,
	next: Next,
) -> Response {
	let security = &state.service.cfg.security;
	let mut req = req;

	sanitize_trusted_token_header(req.headers_mut());

	match security.auth_mode.trim() {
		"off" => next.run(req).await,
		"static_keys" => {
			let key = match resolve_auth_key(req.headers(), &security.auth_keys) {
				Ok(key) => key,
				Err(err) => return err.into_response(),
			};

			req.extensions_mut().insert(key.role);

			if let Err(err) = apply_auth_key_context(req.headers_mut(), key) {
				return err.into_response();
			}

			next.run(req).await
		},
		_ => json_error(
			StatusCode::INTERNAL_SERVER_ERROR,
			"INTERNAL_ERROR",
			"Invalid security.auth_mode configuration.",
			None,
		)
		.into_response(),
	}
}

async fn admin_auth_middleware(
	State(state): State<AppState>,
	req: Request<Body>,
	next: Next,
) -> Response {
	let security = &state.service.cfg.security;
	let mut req = req;

	sanitize_trusted_token_header(req.headers_mut());

	match security.auth_mode.trim() {
		"off" => next.run(req).await,
		"static_keys" => {
			let key = match resolve_auth_key(req.headers(), &security.auth_keys) {
				Ok(key) => key,
				Err(err) => return err.into_response(),
			};

			req.extensions_mut().insert(key.role);

			if !matches!(key.role, SecurityAuthRole::Admin | SecurityAuthRole::SuperAdmin) {
				return json_error(
					StatusCode::FORBIDDEN,
					"FORBIDDEN",
					"Admin token required.",
					None,
				)
				.into_response();
			}

			if let Err(err) = apply_auth_key_context(req.headers_mut(), key) {
				return err.into_response();
			}

			next.run(req).await
		},
		_ => json_error(
			StatusCode::INTERNAL_SERVER_ERROR,
			"INTERNAL_ERROR",
			"Invalid security.auth_mode configuration.",
			None,
		)
		.into_response(),
	}
}

async fn health() -> StatusCode {
	StatusCode::OK
}

async fn notes_ingest(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	payload: Result<Json<NotesIngestRequest>, JsonRejection>,
) -> Result<Json<AddNoteResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let role = role.map(|Extension(role)| role);

	if payload.scope.trim() == "org_shared" {
		require_admin_for_org_shared_writes(state.service.cfg.security.auth_mode.as_str(), role)?;
	}
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
	role: Option<Extension<SecurityAuthRole>>,
	payload: Result<Json<EventsIngestRequest>, JsonRejection>,
) -> Result<Json<AddEventResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let role = role.map(|Extension(role)| role);

	if payload.scope.as_deref().map(str::trim) == Some("org_shared") {
		require_admin_for_org_shared_writes(state.service.cfg.security.auth_mode.as_str(), role)?;
	}
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
			ingestion_profile: payload.ingestion_profile,
			messages: payload.messages,
		})
		.await?;

	Ok(Json(response))
}

async fn docs_put(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	payload: Result<Json<DocsPutBody>, JsonRejection>,
) -> Result<Json<DocsPutResponse>, ApiError> {
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
		.docs_put(DocsPutRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope: payload.scope,
			doc_type: payload.doc_type,
			title: payload.title,
			source_ref: payload.source_ref,
			content: payload.content,
		})
		.await?;

	Ok(Json(response))
}

async fn docs_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(doc_id): Path<Uuid>,
) -> Result<Json<DocsGetResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let response = state
		.service
		.docs_get(DocsGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			doc_id,
		})
		.await?;

	Ok(Json(response))
}

async fn docs_search_l0(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<DocsSearchL0Body>, JsonRejection>,
) -> Result<Json<DocsSearchL0Response>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let Json(mut payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let status = payload.status.as_deref().map(str::trim).filter(|status| !status.is_empty());

	if let Some(status) = status {
		let status = status.to_lowercase();

		if !DOC_STATUSES.contains(&status.as_str()) {
			return Err(json_error(
				StatusCode::BAD_REQUEST,
				"INVALID_REQUEST",
				"status must be one of: active|deleted.",
				Some(vec!["$.status".to_string()]),
			));
		}

		payload.status = Some(status);
	}

	let updated_after = parse_optional_rfc3339(payload.updated_after.as_ref(), "$.updated_after")?;
	let updated_before =
		parse_optional_rfc3339(payload.updated_before.as_ref(), "$.updated_before")?;
	let ts_gte = parse_optional_rfc3339(payload.ts_gte.as_ref(), "$.ts_gte")?;
	let ts_lte = parse_optional_rfc3339(payload.ts_lte.as_ref(), "$.ts_lte")?;

	if let (Some(ts_gte), Some(ts_lte)) = (ts_gte, ts_lte)
		&& ts_gte >= ts_lte
	{
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"ts_gte must be earlier than ts_lte.",
			Some(vec!["$.ts_gte".to_string(), "$.ts_lte".to_string()]),
		));
	}
	if let (Some(updated_after), Some(updated_before)) = (updated_after, updated_before)
		&& updated_after >= updated_before
	{
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"updated_after must be earlier than updated_before.",
			Some(vec!["$.updated_after".to_string(), "$.updated_before".to_string()]),
		));
	}

	if payload.query.chars().count() > MAX_QUERY_CHARS {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"Query is too long.",
			Some(vec!["$.query".to_string()]),
		));
	}

	let response = state
		.service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			caller_agent_id: ctx.agent_id,
			read_profile,
			query: payload.query,
			scope: payload.scope,
			status: payload.status,
			doc_type: payload.doc_type,
			agent_id: payload.agent_id,
			thread_id: payload.thread_id,
			updated_after: payload.updated_after,
			updated_before: payload.updated_before,
			ts_gte: payload.ts_gte,
			ts_lte: payload.ts_lte,
			top_k: payload.top_k,
			candidate_k: payload.candidate_k,
			explain: payload.explain,
		})
		.await?;

	Ok(Json(response))
}

async fn docs_excerpts_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<DocsExcerptsGetBody>, JsonRejection>,
) -> Result<Json<DocsExcerptResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let read_profile = required_read_profile(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let response = state
		.service
		.docs_excerpts_get(DocsExcerptsGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			read_profile,
			doc_id: payload.doc_id,
			level: payload.level,
			chunk_id: payload.chunk_id,
			quote: payload.quote,
			position: payload.position,
			explain: payload.explain,
		})
		.await?;

	Ok(Json(response))
}

async fn search_quick_create(
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
		.search_quick(SearchRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			token_id: effective_token_id(state.service.cfg.security.auth_mode.as_str(), &headers),
			read_profile,
			query: payload.query,
			top_k: payload.top_k,
			candidate_k: payload.candidate_k,
			filter: payload.filter,
			payload_level: payload.payload_level.unwrap_or_default(),
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

async fn search_planned_create(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<SearchCreateRequest>, JsonRejection>,
) -> Result<Json<SearchIndexPlannedResponseV2>, ApiError> {
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
		.search_planned(SearchRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			token_id: effective_token_id(state.service.cfg.security.auth_mode.as_str(), &headers),
			read_profile,
			query: payload.query,
			top_k: payload.top_k,
			candidate_k: payload.candidate_k,
			filter: payload.filter,
			payload_level: payload.payload_level.unwrap_or_default(),
			record_hits: Some(false),
			ranking: None,
		})
		.await?;

	Ok(Json(SearchIndexPlannedResponseV2 {
		trace_id: response.trace_id,
		search_id: response.search_session_id,
		expires_at: response.expires_at,
		items: response.items,
		query_plan: response.query_plan,
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
			payload_level: query.payload_level.unwrap_or_default(),
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
			payload_level: query.payload_level.unwrap_or_default(),
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

async fn notes_publish(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	Path(note_id): Path<Uuid>,
	payload: Result<Json<ShareScopeBody>, JsonRejection>,
) -> Result<Json<PublishResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let scope = parse_space(payload.space.as_str())?;
	let role = role.map(|Extension(role)| role);

	if matches!(scope, ShareScope::OrgShared) {
		require_admin_for_org_shared_writes(state.service.cfg.security.auth_mode.as_str(), role)?;
	}

	let response = state
		.service
		.publish_note(PublishNoteRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			note_id,
			scope,
		})
		.await?;

	Ok(Json(PublishResponseV2 {
		note_id: response.note_id,
		space: format_scope(response.scope.as_str())?.to_string(),
	}))
}

async fn notes_unpublish(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	Path(note_id): Path<Uuid>,
	payload: Result<Json<ShareScopeBody>, JsonRejection>,
) -> Result<Json<PublishResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let scope = parse_space(payload.space.as_str())?;
	let role = role.map(|Extension(role)| role);

	if matches!(scope, ShareScope::OrgShared) {
		require_admin_for_org_shared_writes(state.service.cfg.security.auth_mode.as_str(), role)?;
	}

	let response = state
		.service
		.unpublish_note(UnpublishNoteRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			note_id,
		})
		.await?;

	Ok(Json(PublishResponseV2 {
		note_id: response.note_id,
		space: format_scope(response.scope.as_str())?.to_string(),
	}))
}

async fn space_grants_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(space): Path<String>,
) -> Result<Json<SpaceGrantsListResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let scope = parse_space(space.as_str())?;
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
				space: format_space(item.scope).to_string(),
				grantee_kind: item.grantee_kind,
				grantee_agent_id: item.grantee_agent_id,
				granted_by_agent_id: item.granted_by_agent_id,
				granted_at: item.granted_at,
			})
			.collect(),
	}))
}

async fn space_grant_upsert(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	Path(space): Path<String>,
	payload: Result<Json<SpaceGrantUpsertBody>, JsonRejection>,
) -> Result<Json<SpaceGrantUpsertResponseV2>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let scope = parse_space(space.as_str())?;
	let role = role.map(|Extension(role)| role);

	if matches!(scope, ShareScope::OrgShared) {
		require_admin_for_org_shared_writes(state.service.cfg.security.auth_mode.as_str(), role)?;
	}

	let response = state
		.service
		.space_grant_upsert(SpaceGrantUpsertRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope,
			grantee_kind: payload.grantee_kind,
			grantee_agent_id: payload.grantee_agent_id,
		})
		.await?;

	Ok(Json(SpaceGrantUpsertResponseV2 {
		space: format_scope(response.scope.as_str())?.to_string(),
		grantee_kind: response.grantee_kind,
		grantee_agent_id: response.grantee_agent_id,
		granted: response.granted,
	}))
}

async fn space_grant_revoke(
	State(state): State<AppState>,
	headers: HeaderMap,
	role: Option<Extension<SecurityAuthRole>>,
	Path(space): Path<String>,
	payload: Result<Json<SpaceGrantUpsertBody>, JsonRejection>,
) -> Result<Json<SpaceGrantRevokeResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let scope = parse_space(space.as_str())?;
	let role = role.map(|Extension(role)| role);

	if matches!(scope, ShareScope::OrgShared) {
		require_admin_for_org_shared_writes(state.service.cfg.security.auth_mode.as_str(), role)?;
	}

	let response = state
		.service
		.space_grant_revoke(SpaceGrantRevokeRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope,
			grantee_kind: payload.grantee_kind,
			grantee_agent_id: payload.grantee_agent_id,
		})
		.await?;

	Ok(Json(response))
}

async fn admin_graph_predicates_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	query: Result<Query<AdminGraphPredicatesListQuery>, QueryRejection>,
) -> Result<Json<AdminGraphPredicatesListResponse>, ApiError> {
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
		.admin_graph_predicates_list(AdminGraphPredicatesListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			scope: query.scope,
		})
		.await?;

	Ok(Json(response))
}

async fn admin_graph_predicate_patch(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(predicate_id): Path<Uuid>,
	payload: Result<Json<AdminGraphPredicatePatchBody>, JsonRejection>,
) -> Result<Json<AdminGraphPredicateResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let token_id = effective_token_id(state.service.cfg.security.auth_mode.as_str(), &headers);
	let response = state
		.service
		.admin_graph_predicate_patch(AdminGraphPredicatePatchRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			token_id,
			predicate_id,
			status: payload.status,
			cardinality: payload.cardinality,
		})
		.await?;

	Ok(Json(response))
}

async fn admin_graph_predicate_alias_add(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(predicate_id): Path<Uuid>,
	payload: Result<Json<AdminGraphPredicateAliasAddBody>, JsonRejection>,
) -> Result<Json<AdminGraphPredicateAliasesResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
	})?;
	let token_id = effective_token_id(state.service.cfg.security.auth_mode.as_str(), &headers);
	let response = state
		.service
		.admin_graph_predicate_alias_add(AdminGraphPredicateAliasAddRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			token_id,
			predicate_id,
			alias: payload.alias,
		})
		.await?;

	Ok(Json(response))
}

async fn admin_graph_predicate_aliases_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(predicate_id): Path<Uuid>,
) -> Result<Json<AdminGraphPredicateAliasesResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.admin_graph_predicate_aliases_list(AdminGraphPredicateAliasesListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			predicate_id,
		})
		.await?;

	Ok(Json(response))
}

async fn admin_ingestion_profiles_list(
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

async fn admin_ingestion_profile_create(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<AdminIngestionProfileCreateBody>, JsonRejection>,
) -> Result<Json<AdminIngestionProfileResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
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

async fn admin_ingestion_profile_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(profile_id): Path<String>,
	query: Result<Query<AdminIngestionProfileGetQuery>, QueryRejection>,
) -> Result<Json<AdminIngestionProfileResponse>, ApiError> {
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
		.admin_ingestion_profile_get(AdminIngestionProfileGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			profile_id,
			version: query.version,
		})
		.await?;

	Ok(Json(response))
}

async fn admin_ingestion_profile_versions_list(
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

async fn admin_ingestion_profile_default_get(
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

async fn admin_ingestion_profile_default_set(
	State(state): State<AppState>,
	headers: HeaderMap,
	payload: Result<Json<AdminIngestionProfileDefaultSetBody>, JsonRejection>,
) -> Result<Json<AdminIngestionProfileDefaultResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let Json(payload) = payload.map_err(|err| {
		tracing::warn!(error = %err, "Invalid request payload.");

		json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", "Invalid request payload.", None)
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
			token_id: effective_token_id(state.service.cfg.security.auth_mode.as_str(), &headers),
			read_profile,
			query: payload.query,
			filter: payload.filter,
			payload_level: payload.payload_level.unwrap_or_default(),
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

async fn trace_recent_list(
	State(state): State<AppState>,
	headers: HeaderMap,
	query: Result<Query<TraceRecentListQuery>, QueryRejection>,
) -> Result<Json<TraceRecentListResponse>, ApiError> {
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
	let cursor_created_at =
		parse_optional_rfc3339(query.cursor_created_at.as_ref(), "$.cursor_created_at")?;
	let cursor_trace_id = query.cursor_trace_id;
	let created_after = parse_optional_rfc3339(query.created_after.as_ref(), "$.created_after")?;
	let created_before = parse_optional_rfc3339(query.created_before.as_ref(), "$.created_before")?;

	if cursor_created_at.is_some() != cursor_trace_id.is_some() {
		return Err(json_error(
			StatusCode::BAD_REQUEST,
			"INVALID_REQUEST",
			"cursor_created_at and cursor_trace_id must be both set or both omitted.".to_string(),
			Some(vec!["$.cursor_created_at".to_string(), "$.cursor_trace_id".to_string()]),
		));
	}

	let response = state
		.service
		.trace_recent_list(TraceRecentListRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			limit: query.limit,
			cursor_created_at,
			cursor_trace_id,
			agent_id_filter: query.agent_id,
			read_profile: query.read_profile,
			created_after,
			created_before,
		})
		.await?;

	Ok(Json(response))
}

async fn trace_trajectory_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(trace_id): Path<Uuid>,
) -> Result<Json<SearchTrajectoryResponse>, ApiError> {
	let ctx = RequestContext::from_headers(&headers)?;
	let response = state
		.service
		.trace_trajectory_get(TraceTrajectoryGetRequest {
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

async fn trace_bundle_get(
	State(state): State<AppState>,
	headers: HeaderMap,
	Path(trace_id): Path<Uuid>,
	query: Result<Query<TraceBundleGetQuery>, QueryRejection>,
) -> Result<Json<TraceBundleResponse>, ApiError> {
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
		.trace_bundle_get(TraceBundleGetRequest {
			tenant_id: ctx.tenant_id,
			project_id: ctx.project_id,
			agent_id: ctx.agent_id,
			trace_id,
			mode: query.mode.unwrap_or_default(),
			stage_items_limit: query.stage_items_limit,
			candidates_limit: query.candidates_limit,
		})
		.await?;

	Ok(Json(response))
}

#[cfg(test)]
mod tests {
	use crate::routes::{
		HEADER_AGENT_ID, HEADER_AUTHORIZATION, HEADER_PROJECT_ID, HEADER_READ_PROFILE,
		HEADER_TENANT_ID, HEADER_TRUSTED_TOKEN_ID, apply_auth_key_context, effective_token_id,
		require_admin_for_org_shared_writes, resolve_auth_key, sanitize_trusted_token_header,
	};
	use axum::http::HeaderMap;
	use elf_config::{SecurityAuthKey, SecurityAuthRole};

	#[test]
	fn require_admin_for_org_shared_writes_denies_user_in_static_keys_mode() {
		let err = require_admin_for_org_shared_writes("static_keys", Some(SecurityAuthRole::User))
			.expect_err("Expected forbidden error for non-admin role.");

		assert_eq!(err.status, axum::http::StatusCode::FORBIDDEN);
	}

	#[test]
	fn require_admin_for_org_shared_writes_allows_admin_in_static_keys_mode() {
		require_admin_for_org_shared_writes("static_keys", Some(SecurityAuthRole::Admin))
			.expect("Expected admin role to be allowed.");
	}

	#[test]
	fn require_admin_for_org_shared_writes_allows_superadmin_in_static_keys_mode() {
		require_admin_for_org_shared_writes("static_keys", Some(SecurityAuthRole::SuperAdmin))
			.expect("Expected superadmin role to be allowed.");
	}

	#[test]
	fn require_admin_for_org_shared_writes_allows_non_static_keys_auth_mode() {
		require_admin_for_org_shared_writes("off", None)
			.expect("Expected auth_mode != static_keys.");
	}

	#[test]
	fn resolve_auth_key_requires_bearer_header() {
		let headers = HeaderMap::new();
		let keys = vec![SecurityAuthKey {
			token_id: "k1".to_string(),
			token: "secret".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::User,
		}];
		let err = resolve_auth_key(&headers, &keys).expect_err("Expected unauthorized error.");

		assert_eq!(err.status, axum::http::StatusCode::UNAUTHORIZED);
	}

	#[test]
	fn resolve_auth_key_rejects_unknown_token() {
		let keys = vec![SecurityAuthKey {
			token_id: "k1".to_string(),
			token: "secret".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::User,
		}];
		let mut headers = HeaderMap::new();

		headers.insert(HEADER_AUTHORIZATION, "Bearer wrong".parse().expect("invalid header"));

		let err = resolve_auth_key(&headers, &keys)
			.expect_err("Expected unauthorized error for bad key.");

		assert_eq!(err.status, axum::http::StatusCode::UNAUTHORIZED);
	}

	#[test]
	fn resolve_auth_key_rejects_non_bearer_authorization() {
		let keys = vec![SecurityAuthKey {
			token_id: "k1".to_string(),
			token: "secret".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::User,
		}];
		let mut headers = HeaderMap::new();

		headers.insert(HEADER_AUTHORIZATION, "Token secret".parse().expect("invalid header"));

		let err = resolve_auth_key(&headers, &keys)
			.expect_err("Expected unauthorized error for non-bearer authorization.");

		assert_eq!(err.status, axum::http::StatusCode::UNAUTHORIZED);
	}

	#[test]
	fn resolve_auth_key_rejects_lowercase_bearer_prefix() {
		let keys = vec![SecurityAuthKey {
			token_id: "k1".to_string(),
			token: "secret".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::User,
		}];
		let mut headers = HeaderMap::new();

		headers.insert(HEADER_AUTHORIZATION, "bearer secret".parse().expect("invalid header"));

		let err = resolve_auth_key(&headers, &keys)
			.expect_err("Expected unauthorized error for lowercase bearer prefix.");

		assert_eq!(err.status, axum::http::StatusCode::UNAUTHORIZED);
	}

	#[test]
	fn apply_auth_key_context_overrides_headers() {
		let mut headers = HeaderMap::new();

		headers.insert(HEADER_AUTHORIZATION, "Bearer old".parse().expect("invalid header"));
		headers.insert(HEADER_TENANT_ID, "bad-tenant".parse().expect("invalid header"));
		headers.insert(HEADER_PROJECT_ID, "bad-project".parse().expect("invalid header"));
		headers.insert(HEADER_AGENT_ID, "bad-agent".parse().expect("invalid header"));
		headers.insert(HEADER_READ_PROFILE, "private_only".parse().expect("invalid header"));
		headers.insert(HEADER_TRUSTED_TOKEN_ID, "old-id".parse().expect("invalid header"));

		let key = SecurityAuthKey {
			token_id: "k1".to_string(),
			token: "secret".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "all_scopes".to_string(),
			role: SecurityAuthRole::Admin,
		};

		apply_auth_key_context(&mut headers, &key).expect("Expected context injection.");

		assert_eq!(
			headers.get(HEADER_TENANT_ID).and_then(|v| v.to_str().ok()).expect("missing tenant"),
			"t"
		);
		assert_eq!(
			headers.get(HEADER_PROJECT_ID).and_then(|v| v.to_str().ok()).expect("missing project"),
			"p"
		);
		assert_eq!(
			headers.get(HEADER_AGENT_ID).and_then(|v| v.to_str().ok()).expect("missing agent"),
			"a"
		);
		assert_eq!(
			headers
				.get(HEADER_READ_PROFILE)
				.and_then(|v| v.to_str().ok())
				.expect("missing read profile"),
			"all_scopes"
		);
		assert_eq!(
			headers
				.get(HEADER_TRUSTED_TOKEN_ID)
				.and_then(|v| v.to_str().ok())
				.expect("missing trusted token_id"),
			"k1"
		);
	}

	#[test]
	fn apply_auth_key_context_requires_agent_scope() {
		let mut headers = HeaderMap::new();
		let key = SecurityAuthKey {
			token_id: "k1".to_string(),
			token: "secret".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: None,
			read_profile: "all_scopes".to_string(),
			role: SecurityAuthRole::User,
		};
		let err = apply_auth_key_context(&mut headers, &key)
			.expect_err("Expected forbidden error for missing agent_id.");

		assert_eq!(err.status, axum::http::StatusCode::FORBIDDEN);
	}

	#[test]
	fn effective_token_id_ignores_header_when_auth_mode_off() {
		let mut headers = HeaderMap::new();

		headers.insert(HEADER_TRUSTED_TOKEN_ID, "user-supplied".parse().expect("invalid header"));

		assert_eq!(effective_token_id("off", &headers), None);
	}

	#[test]
	fn effective_token_id_uses_header_when_auth_mode_static_keys() {
		let mut headers = HeaderMap::new();

		headers.insert(HEADER_TRUSTED_TOKEN_ID, "k1".parse().expect("invalid header"));

		assert_eq!(effective_token_id("static_keys", &headers), Some("k1".to_string()));
	}

	#[test]
	fn sanitize_trusted_token_header_removes_header() {
		let mut headers = HeaderMap::new();

		headers.insert(HEADER_TRUSTED_TOKEN_ID, "user-supplied".parse().expect("invalid header"));

		sanitize_trusted_token_header(&mut headers);

		assert!(headers.get(HEADER_TRUSTED_TOKEN_ID).is_none());
	}
}
