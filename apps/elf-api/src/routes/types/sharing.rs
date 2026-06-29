use crate::routes::types::{Deserialize, GranteeKind, OffsetDateTime, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct ShareScopeBody {
	pub(in crate::routes) space: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct SpaceGrantUpsertBody {
	pub(in crate::routes) grantee_kind: GranteeKind,
	pub(in crate::routes) grantee_agent_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::routes) struct SpaceGrantUpsertResponseV2 {
	pub(in crate::routes) space: String,
	pub(in crate::routes) grantee_kind: GranteeKind,
	pub(in crate::routes) grantee_agent_id: Option<String>,
	pub(in crate::routes) granted: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::routes) struct SpaceGrantItemV2 {
	pub(in crate::routes) space: String,
	pub(in crate::routes) grantee_kind: GranteeKind,
	pub(in crate::routes) grantee_agent_id: Option<String>,
	pub(in crate::routes) granted_by_agent_id: String,
	pub(in crate::routes) granted_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::routes) struct SpaceGrantsListResponseV2 {
	pub(in crate::routes) grants: Vec<SpaceGrantItemV2>,
}
