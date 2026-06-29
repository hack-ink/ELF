use crate::{Config, Error, Result};

pub(super) fn validate(cfg: &Config) -> Result<()> {
	if cfg.providers.embedding.dimensions == 0 {
		return Err(Error::Validation {
			message: "providers.embedding.dimensions must be greater than zero.".to_string(),
		});
	}
	if cfg.providers.embedding.dimensions != cfg.storage.qdrant.vector_dim {
		return Err(Error::Validation {
			message: "providers.embedding.dimensions must match storage.qdrant.vector_dim."
				.to_string(),
		});
	}

	for (label, key) in [
		("embedding", &cfg.providers.embedding.api_key),
		("rerank", &cfg.providers.rerank.api_key),
		("llm_extractor", &cfg.providers.llm_extractor.api_key),
	] {
		if key.trim().is_empty() {
			return Err(Error::Validation {
				message: format!("Provider {label} api_key must be non-empty."),
			});
		}
	}

	Ok(())
}
