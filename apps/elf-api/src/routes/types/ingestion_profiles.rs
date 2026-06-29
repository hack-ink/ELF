use crate::routes::types::{Deserialize, Serialize, ToSchema, Value};

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct AdminIngestionProfileCreateBody {
	pub(in crate::routes) profile_id: String,
	pub(in crate::routes) version: Option<i32>,
	pub(in crate::routes) profile: Value,
	pub(in crate::routes) created_by: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct AdminIngestionProfileGetQuery {
	pub(in crate::routes) version: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub(in crate::routes) struct AdminIngestionProfileDefaultSetBody {
	pub(in crate::routes) profile_id: String,
	pub(in crate::routes) version: Option<i32>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub(in crate::routes) struct AdminIngestionProfileDefaultResponseV2 {
	pub(in crate::routes) profile_id: String,
	pub(in crate::routes) version: Option<i32>,
	pub(in crate::routes) updated_at: String,
}
