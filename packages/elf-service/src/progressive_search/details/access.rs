use std::collections::hash_set::HashSet;

use time::OffsetDateTime;

use crate::{
	Error, Result,
	access::{self, SharedSpaceGrantKey},
	progressive_search::types::{SearchDetailsError, session::SearchSession},
};
use elf_config::Config;
use elf_storage::models::MemoryNote;

pub(crate) fn resolve_read_scopes(cfg: &Config, profile: &str) -> Result<Vec<String>> {
	match profile {
		"private_only" => Ok(cfg.scopes.read_profiles.private_only.clone()),
		"private_plus_project" => Ok(cfg.scopes.read_profiles.private_plus_project.clone()),
		"all_scopes" => Ok(cfg.scopes.read_profiles.all_scopes.clone()),
		_ => Err(Error::InvalidRequest { message: "Unknown read_profile.".to_string() }),
	}
}

pub(crate) fn validate_search_session_access(
	session: &SearchSession,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
) -> Result<()> {
	if session.tenant_id != tenant_id
		|| session.project_id != project_id
		|| session.agent_id != agent_id
	{
		return Err(Error::InvalidRequest { message: "Unknown search_session_id.".to_string() });
	}

	Ok(())
}

pub(super) fn validate_note_access(
	note: &MemoryNote,
	session: &SearchSession,
	allowed_scopes: &[String],
	shared_grants: &HashSet<SharedSpaceGrantKey>,
	now: OffsetDateTime,
) -> Option<SearchDetailsError> {
	if note.status != "active" {
		return Some(SearchDetailsError {
			code: "NOTE_INACTIVE".to_string(),
			message: "Note is not active.".to_string(),
		});
	}
	if note.expires_at.map(|ts| ts <= now).unwrap_or(false) {
		return Some(SearchDetailsError {
			code: "NOTE_EXPIRED".to_string(),
			message: "Note is expired.".to_string(),
		});
	}
	if !allowed_scopes.iter().any(|scope| scope == &note.scope) {
		return Some(SearchDetailsError {
			code: "SCOPE_DENIED".to_string(),
			message: "Note scope is not allowed for this read_profile.".to_string(),
		});
	}
	if !access::note_read_allowed(
		note,
		session.agent_id.as_str(),
		allowed_scopes,
		shared_grants,
		now,
	) {
		return Some(SearchDetailsError {
			code: "SCOPE_DENIED".to_string(),
			message: "Note scope is not allowed for this read_profile.".to_string(),
		});
	}

	None
}
