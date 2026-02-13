mod error;
mod types;

pub use error::{Error, Result};
pub use types::{
	Chunking, Config, Context, EmbeddingProviderConfig, Lifecycle, LlmProviderConfig, McpContext,
	Memory, Postgres, ProviderConfig, Providers, Qdrant, Ranking, RankingBlend,
	RankingBlendSegment, RankingDiversity, RankingRetrievalSources, ReadProfiles, ScopePrecedence,
	ScopeWriteAllowed, Scopes, Search, SearchCache, SearchDynamic, SearchExpansion, SearchExplain,
	SearchPrefilter, Security, Service, Storage, TtlDays,
};

use std::{fs, path::Path};

pub fn load(path: &Path) -> Result<Config> {
	let raw = fs::read_to_string(path)
		.map_err(|err| Error::ReadConfig { path: path.to_path_buf(), source: err })?;
	let mut cfg: Config = toml::from_str(&raw)
		.map_err(|err| Error::ParseConfig { path: path.to_path_buf(), source: err })?;

	normalize(&mut cfg);
	validate(&cfg)?;

	Ok(cfg)
}

pub fn validate(cfg: &Config) -> Result<()> {
	validate_security(cfg)?;
	validate_service(cfg)?;
	validate_embedding(cfg)?;
	validate_search(cfg)?;
	validate_ranking(cfg)?;
	validate_chunking(cfg)?;
	validate_provider_keys(cfg)?;
	validate_context(cfg)?;
	validate_mcp(cfg)?;

	Ok(())
}

fn validate_security(cfg: &Config) -> Result<()> {
	if !cfg.security.reject_cjk {
		return Err(Error::Validation { message: "security.reject_cjk must be true.".to_string() });
	}

	Ok(())
}

fn validate_service(cfg: &Config) -> Result<()> {
	if cfg.service.mcp_bind.trim().is_empty() {
		return Err(Error::Validation {
			message: "service.mcp_bind must be non-empty.".to_string(),
		});
	}

	Ok(())
}

fn validate_embedding(cfg: &Config) -> Result<()> {
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

	Ok(())
}

fn validate_search(cfg: &Config) -> Result<()> {
	let expansion_mode = cfg.search.expansion.mode.as_str();

	if !matches!(expansion_mode, "off" | "always" | "dynamic") {
		return Err(Error::Validation {
			message: "search.expansion.mode must be one of off, always, or dynamic.".to_string(),
		});
	}
	if cfg.search.expansion.max_queries == 0 {
		return Err(Error::Validation {
			message: "search.expansion.max_queries must be greater than zero.".to_string(),
		});
	}
	if cfg.search.dynamic.min_candidates == 0 {
		return Err(Error::Validation {
			message: "search.dynamic.min_candidates must be greater than zero.".to_string(),
		});
	}
	if cfg.search.dynamic.min_top_score < 0.0 {
		return Err(Error::Validation {
			message: "search.dynamic.min_top_score must be zero or greater.".to_string(),
		});
	}
	if cfg.search.cache.expansion_ttl_days <= 0 {
		return Err(Error::Validation {
			message: "search.cache.expansion_ttl_days must be greater than zero.".to_string(),
		});
	}
	if cfg.search.cache.rerank_ttl_days <= 0 {
		return Err(Error::Validation {
			message: "search.cache.rerank_ttl_days must be greater than zero.".to_string(),
		});
	}

	if let Some(max) = cfg.search.cache.max_payload_bytes
		&& max == 0
	{
		return Err(Error::Validation {
			message: "search.cache.max_payload_bytes must be greater than zero.".to_string(),
		});
	}

	if cfg.search.explain.retention_days <= 0 {
		return Err(Error::Validation {
			message: "search.explain.retention_days must be greater than zero.".to_string(),
		});
	}
	if cfg.search.explain.candidate_retention_days <= 0 {
		return Err(Error::Validation {
			message: "search.explain.candidate_retention_days must be greater than zero."
				.to_string(),
		});
	}
	if cfg.search.explain.candidate_retention_days > cfg.search.explain.retention_days {
		return Err(Error::Validation {
			message:
				"search.explain.candidate_retention_days must be less than or equal to search.explain.retention_days."
					.to_string(),
		});
	}

	match cfg.search.explain.write_mode.trim().to_ascii_lowercase().as_str() {
		"outbox" | "inline" => {},
		other => {
			return Err(Error::Validation {
				message: format!(
					"search.explain.write_mode must be one of: outbox, inline. Got {other}."
				),
			});
		},
	}

	Ok(())
}

fn validate_ranking(cfg: &Config) -> Result<()> {
	if cfg.ranking.tie_breaker_weight < 0.0 {
		return Err(Error::Validation {
			message: "ranking.tie_breaker_weight must be zero or greater.".to_string(),
		});
	}
	if !cfg.ranking.tie_breaker_weight.is_finite() {
		return Err(Error::Validation {
			message: "ranking.tie_breaker_weight must be a finite number.".to_string(),
		});
	}
	if cfg.ranking.recency_tau_days < 0.0 {
		return Err(Error::Validation {
			message: "ranking.recency_tau_days must be zero or greater.".to_string(),
		});
	}
	if !cfg.ranking.recency_tau_days.is_finite() {
		return Err(Error::Validation {
			message: "ranking.recency_tau_days must be a finite number.".to_string(),
		});
	}

	validate_ranking_blend(cfg)?;
	validate_ranking_diversity(cfg)?;
	validate_ranking_retrieval_sources(cfg)?;
	validate_ranking_deterministic(cfg)?;

	Ok(())
}

fn validate_ranking_blend(cfg: &Config) -> Result<()> {
	if cfg.ranking.blend.enabled {
		if cfg.ranking.blend.segments.is_empty() {
			return Err(Error::Validation {
				message: "ranking.blend.segments must be non-empty when enabled.".to_string(),
			});
		}

		for segment in &cfg.ranking.blend.segments {
			if !segment.retrieval_weight.is_finite() {
				return Err(Error::Validation {
					message: "ranking.blend.segments.retrieval_weight must be a finite number."
						.to_string(),
				});
			}
			if !(0.0..=1.0).contains(&segment.retrieval_weight) {
				return Err(Error::Validation {
					message:
						"ranking.blend.segments.retrieval_weight must be in the range 0.0-1.0."
							.to_string(),
				});
			}
			if segment.max_retrieval_rank == 0 {
				return Err(Error::Validation {
					message: "ranking.blend.segments.max_retrieval_rank must be greater than zero."
						.to_string(),
				});
			}
		}
	}

	Ok(())
}

fn validate_ranking_diversity(cfg: &Config) -> Result<()> {
	let diversity = &cfg.ranking.diversity;

	if !diversity.sim_threshold.is_finite() {
		return Err(Error::Validation {
			message: "ranking.diversity.sim_threshold must be a finite number.".to_string(),
		});
	}
	if !(0.0..=1.0).contains(&diversity.sim_threshold) {
		return Err(Error::Validation {
			message: "ranking.diversity.sim_threshold must be in the range 0.0-1.0.".to_string(),
		});
	}
	if !diversity.mmr_lambda.is_finite() {
		return Err(Error::Validation {
			message: "ranking.diversity.mmr_lambda must be a finite number.".to_string(),
		});
	}
	if !(0.0..=1.0).contains(&diversity.mmr_lambda) {
		return Err(Error::Validation {
			message: "ranking.diversity.mmr_lambda must be in the range 0.0-1.0.".to_string(),
		});
	}

	Ok(())
}

fn validate_ranking_retrieval_sources(cfg: &Config) -> Result<()> {
	let retrieval_sources = &cfg.ranking.retrieval_sources;

	for (path, value) in [
		("ranking.retrieval_sources.fusion_weight", retrieval_sources.fusion_weight),
		(
			"ranking.retrieval_sources.structured_field_weight",
			retrieval_sources.structured_field_weight,
		),
	] {
		if !value.is_finite() {
			return Err(Error::Validation { message: format!("{path} must be a finite number.") });
		}
		if value < 0.0 {
			return Err(Error::Validation { message: format!("{path} must be zero or greater.") });
		}
	}

	if retrieval_sources.fusion_weight <= 0.0 && retrieval_sources.structured_field_weight <= 0.0 {
		return Err(Error::Validation {
			message: "At least one retrieval source weight must be greater than zero.".to_string(),
		});
	}

	Ok(())
}

fn validate_ranking_deterministic(cfg: &Config) -> Result<()> {
	let det = &cfg.ranking.deterministic;
	let det_lex = &det.lexical;
	let det_hits = &det.hits;
	let det_decay = &det.decay;

	for (path, weight) in [
		("ranking.deterministic.lexical", det_lex.weight),
		("ranking.deterministic.hits", det_hits.weight),
		("ranking.deterministic.decay", det_decay.weight),
	] {
		if weight < 0.0 {
			return Err(Error::Validation {
				message: format!("{path}.weight must be zero or greater."),
			});
		}
		if !weight.is_finite() {
			return Err(Error::Validation {
				message: format!("{path}.weight must be a finite number."),
			});
		}
	}

	if det.enabled && det_lex.enabled {
		if !det_lex.min_ratio.is_finite() {
			return Err(Error::Validation {
				message: "ranking.deterministic.lexical.min_ratio must be a finite number."
					.to_string(),
			});
		}
		if !(0.0..=1.0).contains(&det_lex.min_ratio) {
			return Err(Error::Validation {
				message: "ranking.deterministic.lexical.min_ratio must be in the range 0.0-1.0."
					.to_string(),
			});
		}
		if det_lex.max_query_terms == 0 {
			return Err(Error::Validation {
				message: "ranking.deterministic.lexical.max_query_terms must be greater than zero."
					.to_string(),
			});
		}
		if det_lex.max_text_terms == 0 {
			return Err(Error::Validation {
				message: "ranking.deterministic.lexical.max_text_terms must be greater than zero."
					.to_string(),
			});
		}
	}
	if det.enabled && det_hits.enabled {
		if !det_hits.half_saturation.is_finite() {
			return Err(Error::Validation {
				message: "ranking.deterministic.hits.half_saturation must be a finite number."
					.to_string(),
			});
		}
		if det_hits.half_saturation <= 0.0 {
			return Err(Error::Validation {
				message: "ranking.deterministic.hits.half_saturation must be greater than zero."
					.to_string(),
			});
		}
		if !det_hits.last_hit_tau_days.is_finite() {
			return Err(Error::Validation {
				message: "ranking.deterministic.hits.last_hit_tau_days must be a finite number."
					.to_string(),
			});
		}
		if det_hits.last_hit_tau_days < 0.0 {
			return Err(Error::Validation {
				message: "ranking.deterministic.hits.last_hit_tau_days must be zero or greater."
					.to_string(),
			});
		}
	}
	if det.enabled && det_decay.enabled {
		if !det_decay.tau_days.is_finite() {
			return Err(Error::Validation {
				message: "ranking.deterministic.decay.tau_days must be a finite number."
					.to_string(),
			});
		}
		if det_decay.tau_days <= 0.0 {
			return Err(Error::Validation {
				message: "ranking.deterministic.decay.tau_days must be greater than zero."
					.to_string(),
			});
		}
	}

	Ok(())
}

fn validate_chunking(cfg: &Config) -> Result<()> {
	if !cfg.chunking.enabled {
		return Err(Error::Validation { message: "chunking.enabled must be true.".to_string() });
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

fn validate_provider_keys(cfg: &Config) -> Result<()> {
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

fn validate_context(cfg: &Config) -> Result<()> {
	if let Some(context) = cfg.context.as_ref()
		&& let Some(weight) = context.scope_boost_weight
	{
		if !weight.is_finite() {
			return Err(Error::Validation {
				message: "context.scope_boost_weight must be a finite number.".to_string(),
			});
		}
		if weight < 0.0 {
			return Err(Error::Validation {
				message: "context.scope_boost_weight must be zero or greater.".to_string(),
			});
		}
		if weight > 1.0 {
			return Err(Error::Validation {
				message: "context.scope_boost_weight must be 1.0 or less.".to_string(),
			});
		}
		if weight > 0.0
			&& context
				.scope_descriptions
				.as_ref()
				.map(|descriptions| descriptions.is_empty())
				.unwrap_or(true)
		{
			return Err(Error::Validation {
				message: "context.scope_descriptions must be non-empty when context.scope_boost_weight is greater than zero."
					.to_string(),
			});
		}
	}

	Ok(())
}

fn validate_mcp(cfg: &Config) -> Result<()> {
	if let Some(mcp) = cfg.mcp.as_ref() {
		for (label, value) in [
			("mcp.tenant_id", &mcp.tenant_id),
			("mcp.project_id", &mcp.project_id),
			("mcp.agent_id", &mcp.agent_id),
			("mcp.read_profile", &mcp.read_profile),
		] {
			if value.trim().is_empty() {
				return Err(Error::Validation { message: format!("{label} must be non-empty.") });
			}
		}

		if !matches!(
			mcp.read_profile.as_str(),
			"private_only" | "private_plus_project" | "all_scopes"
		) {
			return Err(Error::Validation {
				message:
					"mcp.read_profile must be one of private_only, private_plus_project, or all_scopes."
						.to_string(),
			});
		}
	}

	Ok(())
}

fn normalize(cfg: &mut Config) {
	if cfg.chunking.tokenizer_repo.as_deref().map(|repo| repo.trim().is_empty()).unwrap_or(false) {
		cfg.chunking.tokenizer_repo = None;
	}
	if cfg.security.api_auth_token.as_deref().map(|token| token.trim().is_empty()).unwrap_or(false)
	{
		cfg.security.api_auth_token = None;
	}
	if cfg
		.security
		.admin_auth_token
		.as_deref()
		.map(|token| token.trim().is_empty())
		.unwrap_or(false)
	{
		cfg.security.admin_auth_token = None;
	}
}
