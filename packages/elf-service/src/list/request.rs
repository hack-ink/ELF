use crate::{Error, Result, list::ListRequest};

pub(super) fn requested_list_status(requested_status: Option<&String>) -> Option<&str> {
	requested_status.map(|value| value.trim()).filter(|value| !value.is_empty())
}

pub(super) fn validate_list_request(
	req: &ListRequest,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	allowed_scopes: &[String],
) -> Result<()> {
	if tenant_id.is_empty() || project_id.is_empty() {
		return Err(Error::InvalidRequest {
			message: "tenant_id and project_id are required.".to_string(),
		});
	}

	if let Some(scope) = req.scope.as_ref()
		&& !allowed_scopes.iter().any(|value| value == scope)
	{
		return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
	}
	if let Some(agent_id) = req.agent_id.as_ref()
		&& agent_id.trim().is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "agent_id must not be empty when provided.".to_string(),
		});
	}

	if req.scope.as_deref() == Some("agent_private") && agent_id.is_empty() {
		return Err(Error::ScopeDenied {
			message: "agent_id is required for agent_private scope.".to_string(),
		});
	}

	Ok(())
}
