use crate::{Error, Result, access::ORG_PROJECT_ID, consolidation::types::PromotedMemoryPayload};

pub(in crate::consolidation) fn promoted_memory_scope(
	payload: &PromotedMemoryPayload,
	default_scope: &str,
) -> Result<String> {
	match payload.scope.as_deref() {
		Some(raw) => {
			let scope = raw.trim();

			if scope.is_empty() {
				return Err(Error::InvalidRequest {
					message: "proposed_payload.scope must not be empty when provided.".to_string(),
				});
			}

			Ok(scope.to_string())
		},
		None => Ok(default_scope.to_string()),
	}
}

pub(in crate::consolidation) fn promoted_memory_project_id<'a>(
	proposal_project_id: &'a str,
	scope: &str,
) -> &'a str {
	if scope == "org_shared" { ORG_PROJECT_ID } else { proposal_project_id }
}

pub(in crate::consolidation) fn normalized_optional_string(
	value: Option<String>,
) -> Option<String> {
	value.map(|raw| raw.trim().to_string()).filter(|trimmed| !trimmed.is_empty())
}
