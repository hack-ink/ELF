use super::types::{EntityMemoryViewRequest, PreparedEntityMemoryRequest};
use crate::{Error, Result};

pub(super) fn validate_entity_memory_request(
	req: EntityMemoryViewRequest,
) -> Result<PreparedEntityMemoryRequest> {
	let tenant_id = normalize_required(req.tenant_id.as_str(), "tenant_id")?;
	let project_id = normalize_required(req.project_id.as_str(), "project_id")?;
	let agent_id = normalize_required(req.agent_id.as_str(), "agent_id")?;
	let read_profile = normalize_required(req.read_profile.as_str(), "read_profile")?;
	let entity_surface = req
		.entity_surface
		.as_deref()
		.map(|surface| normalize_required(surface, "entity_surface"))
		.transpose()?;

	if req.entity_id.is_some() == entity_surface.is_some() {
		return Err(Error::InvalidRequest {
			message: "Exactly one of entity_id or entity_surface is required.".to_string(),
		});
	}

	Ok(PreparedEntityMemoryRequest {
		tenant_id,
		project_id,
		agent_id,
		read_profile,
		entity_id: req.entity_id,
		entity_surface,
	})
}

fn normalize_required(raw: &str, field: &str) -> Result<String> {
	let trimmed = raw.trim();

	if trimmed.is_empty() {
		return Err(Error::InvalidRequest { message: format!("{field} is required.") });
	}

	Ok(trimmed.to_string())
}
