use crate::routes::{ApiError, AppState, ErrorBody, Json, RebuildReport, State};

#[utoipa::path(
	post,
	path = "/v2/admin/qdrant/rebuild",
	tag = "admin",
	responses(
		(status = 200, description = "Qdrant rebuild report.", body = Value),
		(status = 401, description = "Authentication required.", body = ErrorBody),
		(status = 403, description = "Admin access required.", body = ErrorBody),
		(status = 500, description = "Internal error.", body = ErrorBody),
	)
)]
pub(super) async fn rebuild_qdrant(
	State(state): State<AppState>,
) -> Result<Json<RebuildReport>, ApiError> {
	let response = state.service.rebuild_qdrant().await?;

	Ok(Json(response))
}
