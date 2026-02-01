use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;

use crate::state::{
    AddEventRequest, AddEventResponse, AddNoteRequest, AddNoteResponse, AppState, DeleteRequest,
    DeleteResponse, ListResponse, RebuildReport, SearchRequest, SearchResponse, ServiceError,
    UpdateRequest, UpdateResponse,
};

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
    Json(payload): Json<AddNoteRequest>,
) -> Result<Json<AddNoteResponse>, ApiError> {
    let response = state.service.add_note(payload).await?;
    Ok(Json(response))
}

async fn add_event(
    State(state): State<AppState>,
    Json(payload): Json<AddEventRequest>,
) -> Result<Json<AddEventResponse>, ApiError> {
    let response = state.service.add_event(payload).await?;
    Ok(Json(response))
}

async fn search(
    State(state): State<AppState>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, ApiError> {
    let response = state.service.search(payload).await?;
    Ok(Json(response))
}

async fn list(State(state): State<AppState>) -> Result<Json<ListResponse>, ApiError> {
    let response = state.service.list().await?;
    Ok(Json(response))
}

async fn update(
    State(state): State<AppState>,
    Json(payload): Json<UpdateRequest>,
) -> Result<Json<UpdateResponse>, ApiError> {
    let response = state.service.update(payload).await?;
    Ok(Json(response))
}

async fn delete(
    State(state): State<AppState>,
    Json(payload): Json<DeleteRequest>,
) -> Result<Json<DeleteResponse>, ApiError> {
    let response = state.service.delete(payload).await?;
    Ok(Json(response))
}

async fn rebuild_qdrant(
    State(state): State<AppState>,
) -> Result<Json<RebuildReport>, ApiError> {
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
            ServiceError::NotImplemented { operation } => json_error(
                StatusCode::NOT_IMPLEMENTED,
                "not_implemented",
                format!("{operation} is not implemented."),
                None,
            ),
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
