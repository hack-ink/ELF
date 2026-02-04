mod types;

pub use types::*;

pub fn load(path: &std::path::Path) -> color_eyre::Result<Config> {
	let raw = std::fs::read_to_string(path)?;
	let cfg: Config = toml::from_str(&raw)?;
	validate(&cfg)?;
	Ok(cfg)
}

pub fn validate(cfg: &Config) -> color_eyre::Result<()> {
	if !cfg.security.reject_cjk {
		return Err(color_eyre::eyre::eyre!("security.reject_cjk must be true."));
	}
	if cfg.service.mcp_bind.trim().is_empty() {
		return Err(color_eyre::eyre::eyre!("service.mcp_bind must be non-empty."));
	}
	if cfg.providers.embedding.dimensions == 0 {
		return Err(color_eyre::eyre::eyre!(
			"providers.embedding.dimensions must be greater than zero."
		));
	}
	if cfg.providers.embedding.dimensions != cfg.storage.qdrant.vector_dim {
		return Err(color_eyre::eyre::eyre!(
			"providers.embedding.dimensions must match storage.qdrant.vector_dim."
		));
	}
	let expansion_mode = cfg.search.expansion.mode.as_str();
	if !matches!(expansion_mode, "off" | "always" | "dynamic") {
		return Err(color_eyre::eyre::eyre!(
			"search.expansion.mode must be one of off, always, or dynamic."
		));
	}
	if cfg.search.expansion.max_queries == 0 {
		return Err(color_eyre::eyre::eyre!(
			"search.expansion.max_queries must be greater than zero."
		));
	}
	if cfg.search.dynamic.min_candidates == 0 {
		return Err(color_eyre::eyre::eyre!(
			"search.dynamic.min_candidates must be greater than zero."
		));
	}
	if cfg.search.dynamic.min_top_score < 0.0 {
		return Err(color_eyre::eyre::eyre!(
			"search.dynamic.min_top_score must be zero or greater."
		));
	}
	if cfg.search.explain.retention_days <= 0 {
		return Err(color_eyre::eyre::eyre!(
			"search.explain.retention_days must be greater than zero."
		));
	}
	for (label, key) in [
		("embedding", &cfg.providers.embedding.api_key),
		("rerank", &cfg.providers.rerank.api_key),
		("llm_extractor", &cfg.providers.llm_extractor.api_key),
	] {
		if key.trim().is_empty() {
			return Err(color_eyre::eyre::eyre!("Provider {label} api_key must be non-empty."));
		}
	}
	Ok(())
}
