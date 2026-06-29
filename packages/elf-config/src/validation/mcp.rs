use crate::{Config, Error, Result};

pub(super) fn validate(cfg: &Config) -> Result<()> {
	let Some(mcp) = cfg.mcp.as_ref() else { return Ok(()) };

	for (label, value) in [
		("mcp.tenant_id", &mcp.tenant_id),
		("mcp.project_id", &mcp.project_id),
		("mcp.agent_id", &mcp.agent_id),
		("mcp.read_profile", &mcp.read_profile),
	] {
		if value.trim().is_empty() {
			return Err(Error::Validation { message: format!("{label} must be non-empty.") });
		}
	}

	if !matches!(mcp.read_profile.as_str(), "private_only" | "private_plus_project" | "all_scopes")
	{
		return Err(Error::Validation {
			message:
				"mcp.read_profile must be one of private_only, private_plus_project, or all_scopes."
					.to_string(),
		});
	}

	Ok(())
}
