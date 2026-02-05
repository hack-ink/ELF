mod types;

// std
use std::{fs, path::Path};

// crates.io
use color_eyre::eyre;

// self
pub use types::{
	Chunking, Config, EmbeddingProviderConfig, Lifecycle, LlmProviderConfig, Memory, Postgres,
	ProviderConfig, Providers, Qdrant, Ranking, ReadProfiles, ScopePrecedence, ScopeWriteAllowed,
	Scopes, Search, SearchCache, SearchDynamic, SearchExpansion, SearchExplain, SearchPrefilter,
	Security, Service, Storage, TtlDays,
};

pub fn load(path: &Path) -> color_eyre::Result<Config> {
	let raw = fs::read_to_string(path)?;
	let mut cfg: Config = toml::from_str(&raw)?;
	normalize(&mut cfg);
	validate(&cfg)?;
	Ok(cfg)
}

fn normalize(cfg: &mut Config) {
	if cfg.chunking.tokenizer_repo.as_deref().map(|repo| repo.trim().is_empty()).unwrap_or(false) {
		cfg.chunking.tokenizer_repo = None;
	}
}

pub fn validate(cfg: &Config) -> color_eyre::Result<()> {
	if !cfg.security.reject_cjk {
		return Err(eyre::eyre!("security.reject_cjk must be true."));
	}
	if cfg.service.mcp_bind.trim().is_empty() {
		return Err(eyre::eyre!("service.mcp_bind must be non-empty."));
	}
	if cfg.providers.embedding.dimensions == 0 {
		return Err(eyre::eyre!("providers.embedding.dimensions must be greater than zero."));
	}
	if cfg.providers.embedding.dimensions != cfg.storage.qdrant.vector_dim {
		return Err(eyre::eyre!(
			"providers.embedding.dimensions must match storage.qdrant.vector_dim."
		));
	}
	let expansion_mode = cfg.search.expansion.mode.as_str();
	if !matches!(expansion_mode, "off" | "always" | "dynamic") {
		return Err(eyre::eyre!("search.expansion.mode must be one of off, always, or dynamic."));
	}
	if cfg.search.expansion.max_queries == 0 {
		return Err(eyre::eyre!("search.expansion.max_queries must be greater than zero."));
	}
	if cfg.search.dynamic.min_candidates == 0 {
		return Err(eyre::eyre!("search.dynamic.min_candidates must be greater than zero."));
	}
	if cfg.search.dynamic.min_top_score < 0.0 {
		return Err(eyre::eyre!("search.dynamic.min_top_score must be zero or greater."));
	}
	if cfg.search.cache.expansion_ttl_days <= 0 {
		return Err(eyre::eyre!("search.cache.expansion_ttl_days must be greater than zero."));
	}
	if cfg.search.cache.rerank_ttl_days <= 0 {
		return Err(eyre::eyre!("search.cache.rerank_ttl_days must be greater than zero."));
	}
	if let Some(max) = cfg.search.cache.max_payload_bytes
		&& max == 0
	{
		return Err(eyre::eyre!("search.cache.max_payload_bytes must be greater than zero."));
	}
	if cfg.search.cache.expansion_version.trim().is_empty() {
		return Err(eyre::eyre!("search.cache.expansion_version must be non-empty."));
	}
	if cfg.search.cache.rerank_version.trim().is_empty() {
		return Err(eyre::eyre!("search.cache.rerank_version must be non-empty."));
	}
	if cfg.search.explain.retention_days <= 0 {
		return Err(eyre::eyre!("search.explain.retention_days must be greater than zero."));
	}
	if !cfg.chunking.enabled {
		return Err(eyre::eyre!("chunking.enabled must be true."));
	}
	if cfg.chunking.max_tokens == 0 {
		return Err(eyre::eyre!("chunking.max_tokens must be greater than zero."));
	}
	if cfg.chunking.overlap_tokens >= cfg.chunking.max_tokens {
		return Err(eyre::eyre!("chunking.overlap_tokens must be less than chunking.max_tokens."));
	}
	for (label, key) in [
		("embedding", &cfg.providers.embedding.api_key),
		("rerank", &cfg.providers.rerank.api_key),
		("llm_extractor", &cfg.providers.llm_extractor.api_key),
	] {
		if key.trim().is_empty() {
			return Err(eyre::eyre!("Provider {label} api_key must be non-empty."));
		}
	}
	Ok(())
}
