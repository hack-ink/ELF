use time::OffsetDateTime;

use crate::{
	ElfService, Error, Result,
	access::ORG_PROJECT_ID,
	sharing::types::{GranteeKind, SpaceGrantRevokeRequest, SpaceGrantRevokeResponse},
};

impl ElfService {
	/// Revokes a shared-scope grant.
	pub async fn space_grant_revoke(
		&self,
		req: SpaceGrantRevokeRequest,
	) -> Result<SpaceGrantRevokeResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let scope = req.scope.as_str();
		let grantee_agent_id = req
			.grantee_agent_id
			.as_deref()
			.map(|value| value.trim())
			.filter(|value| !value.is_empty());

		if req.grantee_kind == GranteeKind::Agent && grantee_agent_id.is_none() {
			return Err(Error::InvalidRequest {
				message: "grantee_agent_id is required for agent grantee_kind.".to_string(),
			});
		}
		if req.grantee_kind == GranteeKind::Project && grantee_agent_id.is_some() {
			return Err(Error::InvalidRequest {
				message: "grantee_agent_id must be empty for project grantee_kind.".to_string(),
			});
		}

		let scope_allowed = match scope {
			"project_shared" => self.cfg.scopes.write_allowed.project_shared,
			"org_shared" => self.cfg.scopes.write_allowed.org_shared,
			_ => false,
		};

		if !scope_allowed {
			return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
		}

		let effective_project_id = if scope == "org_shared" { ORG_PROJECT_ID } else { project_id };
		let revocation = sqlx::query(
			"\
UPDATE memory_space_grants
SET revoked_at = $7,
	revoked_by_agent_id = $8
WHERE tenant_id = $1
  AND project_id = $2
  AND scope = $3
  AND space_owner_agent_id = $4
  AND grantee_kind = $5
  AND ((grantee_kind = 'project' AND grantee_agent_id IS NULL)
  OR (grantee_kind = 'agent' AND grantee_agent_id = $6))
  AND revoked_at IS NULL",
		)
		.bind(tenant_id)
		.bind(effective_project_id)
		.bind(scope)
		.bind(agent_id)
		.bind(match req.grantee_kind {
			GranteeKind::Project => "project",
			GranteeKind::Agent => "agent",
		})
		.bind(grantee_agent_id)
		.bind(OffsetDateTime::now_utc())
		.bind(agent_id)
		.execute(&self.db.pool)
		.await?;

		if revocation.rows_affected() == 0 {
			return Err(Error::InvalidRequest { message: "No active grant found.".to_string() });
		}

		Ok(SpaceGrantRevokeResponse { revoked: true })
	}
}
