mod admin;
mod profile;
mod resolution;
mod storage;
mod types;

const ADD_EVENT_PIPELINE: &str = "add_event";
const DEFAULT_PROFILE_ID: &str = "default";
const DEFAULT_PROFILE_VERSION: i32 = 1;

pub(crate) use resolution::resolve_add_event_profile;
pub use types::{
	AdminIngestionProfileCreateRequest, AdminIngestionProfileDefaultGetRequest,
	AdminIngestionProfileDefaultResponse, AdminIngestionProfileDefaultSetRequest,
	AdminIngestionProfileGetRequest, AdminIngestionProfileListRequest,
	AdminIngestionProfileResponse, AdminIngestionProfileSummary,
	AdminIngestionProfileVersionsListRequest, AdminIngestionProfileVersionsListResponse,
	AdminIngestionProfilesListResponse, IngestionProfileRef, IngestionProfileSelector,
};
