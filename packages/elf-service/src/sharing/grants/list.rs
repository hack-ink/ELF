use crate::{
	ElfService, Error, Result,
	access::ORG_PROJECT_ID,
	sharing::{
		grants::rows::SpaceGrantRow,
		types::{
			GranteeKind, ShareScope, SpaceGrantItem, SpaceGrantsListRequest,
			SpaceGrantsListResponse,
		},
	},
};

impl ElfService {
	/// Lists active grants for a shared scope.
	pub async fn space_grants_list(
		&self,
		req: SpaceGrantsListRequest,
	) -> Result<SpaceGrantsListResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let scope = req.scope.as_str();
		let scope_allowed = match scope {
			"project_shared" => self.cfg.scopes.write_allowed.project_shared,
			"org_shared" => self.cfg.scopes.write_allowed.org_shared,
			_ => false,
		};

		if !scope_allowed {
			return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
		}

		let effective_project_id = if scope == "org_shared" { ORG_PROJECT_ID } else { project_id };
		let rows = sqlx::query_as::<_, SpaceGrantRow>(
			"\
SELECT scope, grantee_kind, grantee_agent_id, granted_by_agent_id, granted_at
FROM memory_space_grants
WHERE tenant_id = $1
  AND project_id = $2
  AND space_owner_agent_id = $3
  AND scope = $4
  AND revoked_at IS NULL
ORDER BY granted_at DESC",
		)
		.bind(tenant_id)
		.bind(effective_project_id)
		.bind(agent_id)
		.bind(scope)
		.fetch_all(&self.db.pool)
		.await?;
		let mut grants = Vec::with_capacity(rows.len());

		for row in rows {
			let grantee_kind = match row.grantee_kind.as_str() {
				"agent" => GranteeKind::Agent,
				"project" => GranteeKind::Project,
				_ => continue,
			};
			let scope = match row.scope.as_str() {
				"project_shared" => ShareScope::ProjectShared,
				"org_shared" => ShareScope::OrgShared,
				_ => continue,
			};

			grants.push(SpaceGrantItem {
				scope,
				grantee_kind,
				grantee_agent_id: row.grantee_agent_id,
				granted_by_agent_id: row.granted_by_agent_id,
				granted_at: row.granted_at,
			});
		}

		Ok(SpaceGrantsListResponse { grants })
	}
}
