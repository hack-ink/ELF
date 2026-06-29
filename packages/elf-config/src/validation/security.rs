use std::collections::HashSet;

use crate::{Config, Error, Result};

pub(super) fn validate(cfg: &Config) -> Result<()> {
	if !cfg.security.reject_non_english {
		return Err(Error::Validation {
			message: "security.reject_non_english must be true.".to_string(),
		});
	}

	let auth_mode = cfg.security.auth_mode.trim();

	if !matches!(auth_mode, "off" | "static_keys") {
		return Err(Error::Validation {
			message: "security.auth_mode must be one of off or static_keys.".to_string(),
		});
	}
	if auth_mode == "off" {
		if !cfg.security.auth_keys.is_empty() {
			return Err(Error::Validation {
				message: "security.auth_keys must be empty when security.auth_mode is off."
					.to_string(),
			});
		}

		return Ok(());
	}
	if cfg.security.auth_keys.is_empty() {
		return Err(Error::Validation {
			message: "security.auth_keys must be non-empty when security.auth_mode is static_keys."
				.to_string(),
		});
	}

	let mut token_ids = HashSet::new();
	let mut tokens = HashSet::new();

	for (idx, key) in cfg.security.auth_keys.iter().enumerate() {
		let path = format!("security.auth_keys[{idx}]");

		if key.token_id.trim().is_empty() {
			return Err(Error::Validation {
				message: format!("{path}.token_id must be non-empty."),
			});
		}
		if key.token.trim().is_empty() {
			return Err(Error::Validation { message: format!("{path}.token must be non-empty.") });
		}
		if key.tenant_id.trim().is_empty() {
			return Err(Error::Validation {
				message: format!("{path}.tenant_id must be non-empty."),
			});
		}
		if key.project_id.trim().is_empty() {
			return Err(Error::Validation {
				message: format!("{path}.project_id must be non-empty."),
			});
		}
		if key.read_profile.trim().is_empty() {
			return Err(Error::Validation {
				message: format!("{path}.read_profile must be non-empty."),
			});
		}
		if !matches!(
			key.read_profile.as_str(),
			"private_only" | "private_plus_project" | "all_scopes"
		) {
			return Err(Error::Validation {
				message: format!(
					"{path}.read_profile must be one of private_only, private_plus_project, or all_scopes."
				),
			});
		}

		if let Some(agent_id) = key.agent_id.as_ref()
			&& agent_id.trim().is_empty()
		{
			return Err(Error::Validation {
				message: format!("{path}.agent_id must be non-empty when provided."),
			});
		}

		if key.agent_id.as_ref().map(|agent_id| agent_id.trim().is_empty()).unwrap_or(true) {
			return Err(Error::Validation {
				message: format!(
					"{path}.agent_id is required when security.auth_mode is static_keys."
				),
			});
		}
		if !token_ids.insert(key.token_id.as_str()) {
			return Err(Error::Validation {
				message: format!("{path}.token_id must be unique across security.auth_keys."),
			});
		}
		if !tokens.insert(key.token.as_str()) {
			return Err(Error::Validation {
				message: format!("{path}.token must be unique across security.auth_keys."),
			});
		}
	}

	Ok(())
}
