use crate::{Error, Result, add_event::types::AddEventRequest};
use elf_domain::english_gate;

pub(in crate::add_event) fn validate_add_event_request(req: &AddEventRequest) -> Result<()> {
	if req.messages.is_empty() {
		return Err(Error::InvalidRequest { message: "Messages list is empty.".to_string() });
	}
	if req.tenant_id.trim().is_empty()
		|| req.project_id.trim().is_empty()
		|| req.agent_id.trim().is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "tenant_id, project_id, and agent_id are required.".to_string(),
		});
	}

	if let Some(scope) = req.scope.as_ref()
		&& scope.trim().is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "scope must not be empty when provided.".to_string(),
		});
	}
	if let Some(profile) = req.ingestion_profile.as_ref() {
		if profile.id.trim().is_empty() {
			return Err(Error::InvalidRequest {
				message: "ingestion_profile.id must not be empty.".to_string(),
			});
		}

		if let Some(version) = profile.version
			&& version <= 0
		{
			return Err(Error::InvalidRequest {
				message: "ingestion_profile.version must be greater than zero.".to_string(),
			});
		}
	}

	for (idx, msg) in req.messages.iter().enumerate() {
		if !english_gate::is_english_natural_language(msg.content.as_str()) {
			return Err(Error::NonEnglishInput { field: format!("$.messages[{idx}].content") });
		}
	}

	Ok(())
}
