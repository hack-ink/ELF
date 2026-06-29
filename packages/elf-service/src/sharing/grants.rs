use sqlx::FromRow;
use uuid::Uuid;

use crate::{
	ElfService, Error, Result,
	access::ORG_PROJECT_ID,
	sharing::{
		sql::{AGENT_SPACE_GRANT_UPSERT_SQL, PROJECT_SPACE_GRANT_UPSERT_SQL},
		types::{
			GranteeKind, ShareScope, SpaceGrantItem, SpaceGrantRevokeRequest,
			SpaceGrantRevokeResponse, SpaceGrantUpsertRequest, SpaceGrantUpsertResponse,
			SpaceGrantsListRequest, SpaceGrantsListResponse,
		},
	},
};

impl ElfService {
	/// Creates or reactivates a shared-scope grant.
	pub async fn space_grant_upsert(
		&self,
		req: SpaceGrantUpsertRequest,
	) -> Result<SpaceGrantUpsertResponse> {
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
		if req.grantee_kind == GranteeKind::Agent
			&& req.grantee_agent_id.as_ref().is_none_or(|id| id.trim().is_empty())
		{
			return Err(Error::InvalidRequest {
				message: "grantee_agent_id is required for agent grantee_kind.".to_string(),
			});
		}

		let grantee_agent_id = req
			.grantee_agent_id
			.as_ref()
			.map(|value| value.trim())
			.filter(|value| !value.is_empty())
			.map(ToString::to_string);

		if req.grantee_kind == GranteeKind::Project && grantee_agent_id.is_some() {
			return Err(Error::InvalidRequest {
				message: "grantee_agent_id must be empty for project grantee_kind.".to_string(),
			});
		}

		let grantee_agent_id_ref = grantee_agent_id.as_deref();
		let now = time::OffsetDateTime::now_utc();
		let effective_project_id = if scope == "org_shared" { ORG_PROJECT_ID } else { project_id };

		if req.grantee_kind == GranteeKind::Project {
			self.upsert_project_grant(tenant_id, effective_project_id, scope, agent_id, now)
				.await?;
		} else {
			self.upsert_agent_grant(
				tenant_id,
				effective_project_id,
				scope,
				agent_id,
				grantee_agent_id_ref,
				now,
			)
			.await?;
		}

		Ok(SpaceGrantUpsertResponse {
			scope: scope.to_string(),
			grantee_kind: req.grantee_kind,
			grantee_agent_id,
			granted: true,
		})
	}

	async fn upsert_project_grant(
		&self,
		tenant_id: &str,
		project_id: &str,
		scope: &str,
		agent_id: &str,
		now: time::OffsetDateTime,
	) -> Result<()> {
		sqlx::query(PROJECT_SPACE_GRANT_UPSERT_SQL)
			.bind(Uuid::new_v4())
			.bind(tenant_id)
			.bind(project_id)
			.bind(scope)
			.bind(agent_id)
			.bind("project")
			.bind::<Option<&str>>(None)
			.bind(agent_id)
			.bind(now)
			.execute(&self.db.pool)
			.await?;

		Ok(())
	}

	async fn upsert_agent_grant(
		&self,
		tenant_id: &str,
		project_id: &str,
		scope: &str,
		agent_id: &str,
		grantee_agent_id: Option<&str>,
		now: time::OffsetDateTime,
	) -> Result<()> {
		sqlx::query(AGENT_SPACE_GRANT_UPSERT_SQL)
			.bind(Uuid::new_v4())
			.bind(tenant_id)
			.bind(project_id)
			.bind(scope)
			.bind(agent_id)
			.bind("agent")
			.bind(grantee_agent_id)
			.bind(agent_id)
			.bind(now)
			.execute(&self.db.pool)
			.await?;

		Ok(())
	}

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
		.bind(time::OffsetDateTime::now_utc())
		.bind(agent_id)
		.execute(&self.db.pool)
		.await?;

		if revocation.rows_affected() == 0 {
			return Err(Error::InvalidRequest { message: "No active grant found.".to_string() });
		}

		Ok(SpaceGrantRevokeResponse { revoked: true })
	}

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

		#[derive(FromRow)]
		struct Row {
			scope: String,
			grantee_kind: String,
			grantee_agent_id: Option<String>,
			granted_by_agent_id: String,
			granted_at: time::OffsetDateTime,
		}

		let rows = sqlx::query_as::<_, Row>(
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
