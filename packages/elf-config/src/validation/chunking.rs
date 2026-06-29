use crate::{Config, Error, Result};

pub(super) fn validate(cfg: &Config) -> Result<()> {
	if !cfg.chunking.enabled {
		return Err(Error::Validation { message: "chunking.enabled must be true.".to_string() });
	}
	if cfg.chunking.tokenizer_repo.trim().is_empty() {
		return Err(Error::Validation {
			message: "chunking.tokenizer_repo must be a non-empty string.".to_string(),
		});
	}
	if cfg.chunking.max_tokens == 0 {
		return Err(Error::Validation {
			message: "chunking.max_tokens must be greater than zero.".to_string(),
		});
	}
	if cfg.chunking.overlap_tokens >= cfg.chunking.max_tokens {
		return Err(Error::Validation {
			message: "chunking.overlap_tokens must be less than chunking.max_tokens.".to_string(),
		});
	}

	Ok(())
}
