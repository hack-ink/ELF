use crate::routes::StatusCode;

#[utoipa::path(
	get,
	path = "/health",
	tag = "health",
	responses((status = 200, description = "API process is healthy."))
)]
pub(super) async fn health() -> StatusCode {
	StatusCode::OK
}
