mod admin;
mod profile;
mod resolution;
mod storage;
mod types;

pub use types::{
	AdminIngestionProfileCreateRequest, AdminIngestionProfileDefaultGetRequest,
	AdminIngestionProfileDefaultResponse, AdminIngestionProfileDefaultSetRequest,
	AdminIngestionProfileGetRequest, AdminIngestionProfileListRequest,
	AdminIngestionProfileResponse, AdminIngestionProfileSummary,
	AdminIngestionProfileVersionsListRequest, AdminIngestionProfileVersionsListResponse,
	AdminIngestionProfilesListResponse, IngestionProfileRef, IngestionProfileSelector,
};

use sqlx::PgPool;

use crate::Result;
use types::ResolvedIngestionProfile;

const ADD_EVENT_PIPELINE: &str = "add_event";
const DEFAULT_PROFILE_ID: &str = "default";
const DEFAULT_PROFILE_VERSION: i32 = 1;

pub(crate) async fn resolve_add_event_profile(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	selector: Option<&IngestionProfileSelector>,
) -> Result<ResolvedIngestionProfile> {
	resolution::resolve_add_event_profile(pool, tenant_id, project_id, selector).await
}
