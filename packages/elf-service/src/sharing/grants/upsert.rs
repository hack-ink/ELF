use uuid::Uuid;

use crate::{
	ElfService, Error, Result,
	access::ORG_PROJECT_ID,
	sharing::{
		sql::{AGENT_SPACE_GRANT_UPSERT_SQL, PROJECT_SPACE_GRANT_UPSERT_SQL},
		types::{GranteeKind, SpaceGrantUpsertRequest, SpaceGrantUpsertResponse},
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
}
