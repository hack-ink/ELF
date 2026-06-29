use super::*;

pub(super) fn validate_write_context(
	cfg: &Config,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	scope: &str,
) -> Result<()> {
	if tenant_id.trim().is_empty()
		|| project_id.trim().is_empty()
		|| agent_id.trim().is_empty()
		|| scope.trim().is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "tenant_id, project_id, agent_id, and scope are required.".to_string(),
		});
	}

	validate_identifier(tenant_id, "$.tenant_id")?;
	validate_identifier(project_id, "$.project_id")?;
	validate_identifier(agent_id, "$.agent_id")?;

	if !cfg.scopes.allowed.iter().any(|allowed| allowed == scope.trim()) {
		return Err(Error::ScopeDenied { message: "scope is not allowed.".to_string() });
	}
	if !scope_write_allowed(cfg, scope.trim()) {
		return Err(Error::ScopeDenied { message: "scope is not writable.".to_string() });
	}

	Ok(())
}

pub(in crate::work_journal) fn validate_read_context(
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	read_profile: &str,
) -> Result<()> {
	if tenant_id.trim().is_empty()
		|| project_id.trim().is_empty()
		|| agent_id.trim().is_empty()
		|| read_profile.trim().is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "tenant_id, project_id, agent_id, and read_profile are required.".to_string(),
		});
	}

	validate_identifier(tenant_id, "$.tenant_id")?;
	validate_identifier(project_id, "$.project_id")?;
	validate_identifier(agent_id, "$.agent_id")?;
	validate_identifier(read_profile, "$.read_profile")?;

	Ok(())
}

fn scope_write_allowed(cfg: &Config, scope: &str) -> bool {
	match scope {
		"agent_private" => cfg.scopes.write_allowed.agent_private,
		"project_shared" => cfg.scopes.write_allowed.project_shared,
		"org_shared" => cfg.scopes.write_allowed.org_shared,
		_ => false,
	}
}
