use serde_json::Value;

use crate::{Error, Result};
use elf_config::Config;
use elf_domain::english_gate::{self, EnglishGateKind};

pub(super) fn normalize_required(raw: &str, field: &str) -> Result<String> {
	let trimmed = raw.trim();

	if trimmed.is_empty() {
		return Err(Error::InvalidRequest { message: format!("{field} is required.") });
	}

	Ok(trimmed.to_string())
}

pub(super) fn validate_write_scope(cfg: &Config, scope: &str) -> Result<()> {
	if !cfg.scopes.allowed.iter().any(|allowed| allowed == scope) {
		return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
	}

	let write_allowed = match scope {
		"agent_private" => cfg.scopes.write_allowed.agent_private,
		"project_shared" => cfg.scopes.write_allowed.project_shared,
		"org_shared" => cfg.scopes.write_allowed.org_shared,
		_ => false,
	};

	if !write_allowed {
		return Err(Error::ScopeDenied { message: "Scope is not allowed.".to_string() });
	}

	Ok(())
}

pub(super) fn validate_english(input: &str, kind: EnglishGateKind, field: &str) -> Result<()> {
	english_gate::english_gate(input, kind)
		.map_err(|_| Error::NonEnglishInput { field: field.to_string() })
}

pub(super) fn validate_source_ref(source_ref: &Value) -> Result<()> {
	if !source_ref.is_object() {
		return Err(Error::InvalidRequest {
			message: "source_ref must be a JSON object.".to_string(),
		});
	}

	Ok(())
}
