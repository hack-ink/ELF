use axum::extract::{Query, State};
use axum::extract::rejection::QueryRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;

use crate::state::AppState;
use elf_service::ServiceError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/memory/add_note", post(add_note))
        .route("/v1/memory/add_event", post(add_event))
        .route("/v1/memory/search", post(search))
        .route("/v1/memory/list", get(list))
        .route("/v1/memory/update", post(update))
        .route("/v1/memory/delete", post(delete))
        .with_state(state)
}

pub fn admin_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/admin/rebuild_qdrant", post(rebuild_qdrant))
        .with_state(state)
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn add_note(
    State(state): State<AppState>,
    Json(payload): Json<elf_service::AddNoteRequest>,
) -> Result<Json<elf_service::AddNoteResponse>, ApiError> {
    let response = state.service.add_note(payload).await?;
    Ok(Json(response))
}

async fn add_event(
    State(state): State<AppState>,
    Json(payload): Json<elf_service::AddEventRequest>,
) -> Result<Json<elf_service::AddEventResponse>, ApiError> {
    let response = state.service.add_event(payload).await?;
    Ok(Json(response))
}

async fn search(
    State(state): State<AppState>,
    Json(payload): Json<elf_service::SearchRequest>,
) -> Result<Json<elf_service::SearchResponse>, ApiError> {
    let response = state.service.search(payload).await?;
    Ok(Json(response))
}

async fn list(
    State(state): State<AppState>,
    query: Result<Query<elf_service::ListRequest>, QueryRejection>,
) -> Result<Json<elf_service::ListResponse>, ApiError> {
    let Query(query) = query.map_err(|err| {
        json_error(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            err.to_string(),
            None,
        )
    })?;
    let response = state.service.list(query).await?;
    Ok(Json(response))
}

async fn update(
    State(state): State<AppState>,
    Json(payload): Json<elf_service::UpdateRequest>,
) -> Result<Json<elf_service::UpdateResponse>, ApiError> {
    let response = state.service.update(payload).await?;
    Ok(Json(response))
}

async fn delete(
    State(state): State<AppState>,
    Json(payload): Json<elf_service::DeleteRequest>,
) -> Result<Json<elf_service::DeleteResponse>, ApiError> {
    let response = state.service.delete(payload).await?;
    Ok(Json(response))
}

async fn rebuild_qdrant(
    State(state): State<AppState>,
) -> Result<Json<elf_service::RebuildReport>, ApiError> {
    let response = state.service.rebuild_qdrant().await?;
    Ok(Json(response))
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error_code: String,
    message: String,
    fields: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct ApiError {
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
        Self {
            status,
            error_code: error_code.into(),
            message: message.into(),
            fields,
        }
    }
}

pub fn json_error(
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
    fields: Option<Vec<String>>,
) -> ApiError {
    ApiError::new(status, code, message, fields)
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
            ServiceError::InvalidRequest { message } => {
                json_error(StatusCode::BAD_REQUEST, "INVALID_REQUEST", message, None)
            }
            ServiceError::Provider { message }
            | ServiceError::Storage { message }
            | ServiceError::Qdrant { message } => {
                json_error(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", message, None)
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorBody {
            error_code: self.error_code,
            message: self.message,
            fields: self.fields,
        };
        (self.status, Json(body)).into_response()
    }
}
