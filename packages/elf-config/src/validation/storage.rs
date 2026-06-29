use crate::{Config, Error, Result};

pub(super) fn validate(cfg: &Config) -> Result<()> {
	if cfg.storage.postgres.dsn.trim().is_empty() {
		return Err(Error::Validation {
			message: "storage.postgres.dsn must be non-empty.".to_string(),
		});
	}
	if cfg.storage.qdrant.url.trim().is_empty() {
		return Err(Error::Validation {
			message: "storage.qdrant.url must be non-empty.".to_string(),
		});
	}
	if cfg.storage.qdrant.collection.trim().is_empty() {
		return Err(Error::Validation {
			message: "storage.qdrant.collection must be non-empty.".to_string(),
		});
	}
	if cfg.storage.qdrant.docs_collection.trim().is_empty() {
		return Err(Error::Validation {
			message: "storage.qdrant.docs_collection must be non-empty.".to_string(),
		});
	}
	if cfg.storage.qdrant.vector_dim == 0 {
		return Err(Error::Validation {
			message: "storage.qdrant.vector_dim must be greater than zero.".to_string(),
		});
	}

	Ok(())
}
