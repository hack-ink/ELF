use crate::{Config, Error, Result};

pub(super) fn validate(cfg: &Config) -> Result<()> {
	if cfg.service.mcp_bind.trim().is_empty() {
		return Err(Error::Validation {
			message: "service.mcp_bind must be non-empty.".to_string(),
		});
	}

	Ok(())
}
