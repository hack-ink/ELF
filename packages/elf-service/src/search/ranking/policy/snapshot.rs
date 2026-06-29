use serde_json::Value;

use crate::{
	Error, Result,
	search::{
		RankingRequestOverride,
		ranking::policy::types::{
			ResolvedBlendPolicy, ResolvedDiversityPolicy, ResolvedRetrievalSourcesPolicy,
		},
	},
};
use elf_config::Config;

pub fn build_config_snapshot(
	cfg: &Config,
	blend_policy: &ResolvedBlendPolicy,
	diversity_policy: &ResolvedDiversityPolicy,
	retrieval_sources_policy: &ResolvedRetrievalSourcesPolicy,
	ranking_override: Option<&RankingRequestOverride>,
	policy_id: &str,
	policy_snapshot: &Value,
) -> Value {
	let override_json = ranking_override.and_then(|value| serde_json::to_value(value).ok());

	serde_json::json!({
		"search": {
			"expansion": {
				"mode": cfg.search.expansion.mode.as_str(),
				"max_queries": cfg.search.expansion.max_queries,
				"include_original": cfg.search.expansion.include_original,
			},
			"dynamic": {
				"min_candidates": cfg.search.dynamic.min_candidates,
				"min_top_score": cfg.search.dynamic.min_top_score,
			},
			"prefilter": {
				"max_candidates": cfg.search.prefilter.max_candidates,
			},
			"explain": {
				"retention_days": cfg.search.explain.retention_days,
			},
		},
		"ranking": {
			"policy_id": policy_id,
			"policy_snapshot": policy_snapshot.clone(),
			"recency_tau_days": cfg.ranking.recency_tau_days,
			"tie_breaker_weight": cfg.ranking.tie_breaker_weight,
			"deterministic": {
				"enabled": cfg.ranking.deterministic.enabled,
				"lexical": {
					"enabled": cfg.ranking.deterministic.lexical.enabled,
					"weight": cfg.ranking.deterministic.lexical.weight,
					"min_ratio": cfg.ranking.deterministic.lexical.min_ratio,
					"max_query_terms": cfg.ranking.deterministic.lexical.max_query_terms,
					"max_text_terms": cfg.ranking.deterministic.lexical.max_text_terms,
				},
				"hits": {
					"enabled": cfg.ranking.deterministic.hits.enabled,
					"weight": cfg.ranking.deterministic.hits.weight,
					"half_saturation": cfg.ranking.deterministic.hits.half_saturation,
					"last_hit_tau_days": cfg.ranking.deterministic.hits.last_hit_tau_days,
				},
				"decay": {
					"enabled": cfg.ranking.deterministic.decay.enabled,
					"weight": cfg.ranking.deterministic.decay.weight,
					"tau_days": cfg.ranking.deterministic.decay.tau_days,
				},
			},
				"blend": {
				"enabled": blend_policy.enabled,
				"rerank_normalization": blend_policy.rerank_normalization.as_str(),
				"retrieval_normalization": blend_policy.retrieval_normalization.as_str(),
				"segments": blend_policy
					.segments
					.iter()
					.map(|segment| {
						serde_json::json!({
							"max_retrieval_rank": segment.max_retrieval_rank,
							"retrieval_weight": segment.retrieval_weight,
						})
					})
						.collect::<Vec<_>>(),
				},
				"diversity": {
					"enabled": diversity_policy.enabled,
					"sim_threshold": diversity_policy.sim_threshold,
					"mmr_lambda": diversity_policy.mmr_lambda,
					"max_skips": diversity_policy.max_skips,
				},
				"retrieval_sources": {
					"fusion_weight": retrieval_sources_policy.fusion_weight,
					"structured_field_weight": retrieval_sources_policy.structured_field_weight,
					"recursive_weight": retrieval_sources_policy.recursive_weight,
					"fusion_priority": retrieval_sources_policy.fusion_priority,
					"structured_field_priority": retrieval_sources_policy.structured_field_priority,
					"recursive_priority": retrieval_sources_policy.recursive_priority,
				},
				"override": override_json,
			},
		"providers": {
			"embedding": {
				"provider_id": cfg.providers.embedding.provider_id.as_str(),
				"model": cfg.providers.embedding.model.as_str(),
				"dimensions": cfg.providers.embedding.dimensions,
			},
			"rerank": {
				"provider_id": cfg.providers.rerank.provider_id.as_str(),
				"model": cfg.providers.rerank.model.as_str(),
			},
		},
		"storage": {
			"qdrant": {
				"vector_dim": cfg.storage.qdrant.vector_dim,
				"collection": cfg.storage.qdrant.collection.as_str(),
			},
		},
		"context": {
			"scope_boost_weight": cfg.context.as_ref().and_then(|ctx| ctx.scope_boost_weight),
			"project_description_count": cfg
				.context
				.as_ref()
				.and_then(|ctx| ctx.project_descriptions.as_ref())
				.map(|descriptions| descriptions.len())
				.unwrap_or(0),
			"scope_description_count": cfg
				.context
				.as_ref()
				.and_then(|ctx| ctx.scope_descriptions.as_ref())
				.map(|descriptions| descriptions.len())
				.unwrap_or(0),
		},
	})
}

pub fn build_policy_snapshot(
	cfg: &Config,
	blend_policy: &ResolvedBlendPolicy,
	diversity_policy: &ResolvedDiversityPolicy,
	retrieval_sources_policy: &ResolvedRetrievalSourcesPolicy,
	ranking_override: Option<&RankingRequestOverride>,
) -> Value {
	let override_json = ranking_override.and_then(|value| serde_json::to_value(value).ok());

	serde_json::json!({
		"ranking": {
			"recency_tau_days": cfg.ranking.recency_tau_days,
			"tie_breaker_weight": cfg.ranking.tie_breaker_weight,
			"deterministic": {
				"enabled": cfg.ranking.deterministic.enabled,
				"lexical": {
					"enabled": cfg.ranking.deterministic.lexical.enabled,
					"weight": cfg.ranking.deterministic.lexical.weight,
					"min_ratio": cfg.ranking.deterministic.lexical.min_ratio,
					"max_query_terms": cfg.ranking.deterministic.lexical.max_query_terms,
					"max_text_terms": cfg.ranking.deterministic.lexical.max_text_terms,
				},
				"hits": {
					"enabled": cfg.ranking.deterministic.hits.enabled,
					"weight": cfg.ranking.deterministic.hits.weight,
					"half_saturation": cfg.ranking.deterministic.hits.half_saturation,
					"last_hit_tau_days": cfg.ranking.deterministic.hits.last_hit_tau_days,
				},
				"decay": {
					"enabled": cfg.ranking.deterministic.decay.enabled,
					"weight": cfg.ranking.deterministic.decay.weight,
					"tau_days": cfg.ranking.deterministic.decay.tau_days,
				},
			},
				"blend": {
				"enabled": blend_policy.enabled,
				"rerank_normalization": blend_policy.rerank_normalization.as_str(),
				"retrieval_normalization": blend_policy.retrieval_normalization.as_str(),
				"segments": blend_policy
					.segments
					.iter()
					.map(|segment| {
						serde_json::json!({
							"max_retrieval_rank": segment.max_retrieval_rank,
							"retrieval_weight": segment.retrieval_weight,
						})
					})
						.collect::<Vec<_>>(),
				},
				"diversity": {
					"enabled": diversity_policy.enabled,
					"sim_threshold": diversity_policy.sim_threshold,
					"mmr_lambda": diversity_policy.mmr_lambda,
					"max_skips": diversity_policy.max_skips,
				},
				"retrieval_sources": {
					"fusion_weight": retrieval_sources_policy.fusion_weight,
					"structured_field_weight": retrieval_sources_policy.structured_field_weight,
					"recursive_weight": retrieval_sources_policy.recursive_weight,
					"fusion_priority": retrieval_sources_policy.fusion_priority,
					"structured_field_priority": retrieval_sources_policy.structured_field_priority,
					"recursive_priority": retrieval_sources_policy.recursive_priority,
				},
				"override": override_json,
			},
		"context": {
			"scope_boost_weight": cfg.context.as_ref().and_then(|ctx| ctx.scope_boost_weight),
			"project_description_count": cfg
				.context
				.as_ref()
				.and_then(|ctx| ctx.project_descriptions.as_ref())
				.map(|descriptions| descriptions.len())
				.unwrap_or(0),
			"scope_description_count": cfg
				.context
				.as_ref()
				.and_then(|ctx| ctx.scope_descriptions.as_ref())
				.map(|descriptions| descriptions.len())
				.unwrap_or(0),
		},
	})
}

pub fn hash_policy_snapshot(payload: &Value) -> Result<String> {
	let raw = serde_json::to_vec(payload).map_err(|err| Error::Storage {
		message: format!("Failed to encode policy snapshot: {err}"),
	})?;

	Ok(blake3::hash(&raw).to_hex().to_string())
}
