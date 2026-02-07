mod types;

use std::{fs, path::Path};

use color_eyre::eyre;

pub use types::{
	Chunking, Config, Context, EmbeddingProviderConfig, Lifecycle, LlmProviderConfig, McpContext,
	Memory, Postgres, ProviderConfig, Providers, Qdrant, Ranking, RankingBlend,
	RankingBlendSegment, ReadProfiles, ScopePrecedence, ScopeWriteAllowed, Scopes, Search,
	SearchCache, SearchDynamic, SearchExpansion, SearchExplain, SearchPrefilter, Security, Service,
	Storage, TtlDays,
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

	if cfg.search.explain.retention_days <= 0 {
		return Err(eyre::eyre!("search.explain.retention_days must be greater than zero."));
	}

	if cfg.ranking.tie_breaker_weight < 0.0 {
		return Err(eyre::eyre!("ranking.tie_breaker_weight must be zero or greater."));
	}
	if !cfg.ranking.tie_breaker_weight.is_finite() {
		return Err(eyre::eyre!("ranking.tie_breaker_weight must be a finite number."));
	}
	if cfg.ranking.recency_tau_days < 0.0 {
		return Err(eyre::eyre!("ranking.recency_tau_days must be zero or greater."));
	}
	if !cfg.ranking.recency_tau_days.is_finite() {
		return Err(eyre::eyre!("ranking.recency_tau_days must be a finite number."));
	}
	if cfg.ranking.blend.enabled {
		if cfg.ranking.blend.segments.is_empty() {
			return Err(eyre::eyre!("ranking.blend.segments must be non-empty when enabled."));
		}

		for segment in &cfg.ranking.blend.segments {
			if !segment.retrieval_weight.is_finite() {
				return Err(eyre::eyre!(
					"ranking.blend.segments.retrieval_weight must be a finite number."
				));
			}
			if !(0.0..=1.0).contains(&segment.retrieval_weight) {
				return Err(eyre::eyre!(
					"ranking.blend.segments.retrieval_weight must be in the range 0.0-1.0."
				));
			}
			if segment.max_retrieval_rank == 0 {
				return Err(eyre::eyre!(
					"ranking.blend.segments.max_retrieval_rank must be greater than zero."
				));
			}
		}
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

	if let Some(context) = cfg.context.as_ref()
		&& let Some(weight) = context.scope_boost_weight
	{
		if !weight.is_finite() {
			return Err(eyre::eyre!("context.scope_boost_weight must be a finite number."));
		}
		if weight < 0.0 {
			return Err(eyre::eyre!("context.scope_boost_weight must be zero or greater."));
		}
		if weight > 1.0 {
			return Err(eyre::eyre!("context.scope_boost_weight must be 1.0 or less."));
		}
		if weight > 0.0
			&& context
				.scope_descriptions
				.as_ref()
				.map(|descriptions| descriptions.is_empty())
				.unwrap_or(true)
		{
			return Err(eyre::eyre!(
				"context.scope_descriptions must be non-empty when context.scope_boost_weight is greater than zero."
			));
		}
	}
	if let Some(mcp) = cfg.mcp.as_ref() {
		for (label, value) in [
			("mcp.tenant_id", &mcp.tenant_id),
			("mcp.project_id", &mcp.project_id),
			("mcp.agent_id", &mcp.agent_id),
			("mcp.read_profile", &mcp.read_profile),
		] {
			if value.trim().is_empty() {
				return Err(eyre::eyre!("{label} must be non-empty."));
			}
		}

		if !matches!(
			mcp.read_profile.as_str(),
			"private_only" | "private_plus_project" | "all_scopes"
		) {
			return Err(eyre::eyre!(
				"mcp.read_profile must be one of private_only, private_plus_project, or all_scopes."
			));
		}
	}

	Ok(())
}
