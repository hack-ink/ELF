use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Shareable scopes that can be published or granted.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareScope {
	/// Project-shared scope.
	ProjectShared,
	/// Organization-shared scope.
	OrgShared,
}
impl ShareScope {
	pub(super) fn as_str(&self) -> &'static str {
		match self {
			Self::ProjectShared => "project_shared",
			Self::OrgShared => "org_shared",
		}
	}
}

impl Display for ShareScope {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		self.as_str().fmt(f)
	}
}

/// Grantee classes supported by space grants.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GranteeKind {
	/// Grant the scope to all project readers.
	Project,
	/// Grant the scope to one named agent.
	Agent,
}

/// Request payload for publishing a note into a shared scope.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublishNoteRequest {
	/// Tenant that owns the note.
	pub tenant_id: String,
	/// Project that owns the note.
	pub project_id: String,
	/// Agent requesting the publish operation.
	pub agent_id: String,
	/// Identifier of the note to publish.
	pub note_id: Uuid,
	/// Target shared scope.
	pub scope: ShareScope,
}

/// Response payload for note publishing.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublishNoteResponse {
	/// Identifier of the affected note.
	pub note_id: Uuid,
	/// Effective scope after publishing.
	pub scope: String,
}

/// Request payload for returning a note to its non-shared scope.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UnpublishNoteRequest {
	/// Tenant that owns the note.
	pub tenant_id: String,
	/// Project that owns the note.
	pub project_id: String,
	/// Agent requesting the unpublish operation.
	pub agent_id: String,
	/// Identifier of the note to unpublish.
	pub note_id: Uuid,
}

/// Response payload for note unpublishing.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UnpublishNoteResponse {
	/// Identifier of the affected note.
	pub note_id: Uuid,
	/// Effective scope after unpublishing.
	pub scope: String,
}

/// Request payload for granting a shared scope.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaceGrantUpsertRequest {
	/// Tenant that owns the scope.
	pub tenant_id: String,
	/// Project that owns the scope.
	pub project_id: String,
	/// Agent requesting the grant.
	pub agent_id: String,
	/// Shared scope to grant.
	pub scope: ShareScope,
	/// Grantee class.
	pub grantee_kind: GranteeKind,
	/// Grantee agent identifier when `grantee_kind` is `agent`.
	pub grantee_agent_id: Option<String>,
}

/// Response payload for grant upsert.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaceGrantUpsertResponse {
	/// Granted scope.
	pub scope: String,
	/// Grantee class.
	pub grantee_kind: GranteeKind,
	/// Grantee agent identifier when applicable.
	pub grantee_agent_id: Option<String>,
	/// Whether a grant row is active after the operation.
	pub granted: bool,
}

/// Request payload for revoking a shared-scope grant.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaceGrantRevokeRequest {
	/// Tenant that owns the scope.
	pub tenant_id: String,
	/// Project that owns the scope.
	pub project_id: String,
	/// Agent requesting the revoke operation.
	pub agent_id: String,
	/// Shared scope to revoke.
	pub scope: ShareScope,
	/// Grantee class.
	pub grantee_kind: GranteeKind,
	/// Grantee agent identifier when `grantee_kind` is `agent`.
	pub grantee_agent_id: Option<String>,
}

/// Response payload for grant revocation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaceGrantRevokeResponse {
	/// Whether an active grant was revoked.
	pub revoked: bool,
}

/// Request payload for listing shared-scope grants.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaceGrantsListRequest {
	/// Tenant that owns the scope.
	pub tenant_id: String,
	/// Project that owns the scope.
	pub project_id: String,
	/// Agent requesting the list.
	pub agent_id: String,
	/// Shared scope to inspect.
	pub scope: ShareScope,
}

/// One active space grant returned by `space_grants_list`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaceGrantItem {
	/// Granted scope.
	pub scope: ShareScope,
	/// Grantee class.
	pub grantee_kind: GranteeKind,
	/// Grantee agent identifier when applicable.
	pub grantee_agent_id: Option<String>,
	/// Agent that created the grant.
	pub granted_by_agent_id: String,
	/// Grant creation timestamp.
	pub granted_at: OffsetDateTime,
}

/// Response payload for grant listing.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaceGrantsListResponse {
	/// Active grants visible to the caller.
	pub grants: Vec<SpaceGrantItem>,
}
