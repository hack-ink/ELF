use serde_json::Value;

use crate::{
	Error, Result,
	search::{
		BlendRankingOverride, DiversityRankingOverride, RankingRequestOverride,
		RetrievalSourcesRankingOverride,
	},
};
use elf_config::{Config, RankingBlend, RankingDiversity, RankingRetrievalSources};

#[derive(Clone, Copy, Debug)]
pub enum NormalizationKind {
	Rank,
}
impl NormalizationKind {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Rank => "rank",
		}
	}
}

#[derive(Clone, Debug)]
pub struct BlendSegment {
	pub max_retrieval_rank: u32,
	pub retrieval_weight: f32,
}

#[derive(Clone, Debug)]
pub struct ResolvedBlendPolicy {
	pub enabled: bool,
	pub rerank_normalization: NormalizationKind,
	pub retrieval_normalization: NormalizationKind,
	pub segments: Vec<BlendSegment>,
}

#[derive(Clone, Debug)]
pub struct ResolvedDiversityPolicy {
	pub enabled: bool,
	pub sim_threshold: f32,
	pub mmr_lambda: f32,
	pub max_skips: u32,
}

#[derive(Clone, Debug)]
pub struct ResolvedRetrievalSourcesPolicy {
	pub fusion_weight: f32,
	pub structured_field_weight: f32,
	pub fusion_priority: u32,
	pub structured_field_priority: u32,
}

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
					"fusion_priority": retrieval_sources_policy.fusion_priority,
					"structured_field_priority": retrieval_sources_policy.structured_field_priority,
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
					"fusion_priority": retrieval_sources_policy.fusion_priority,
					"structured_field_priority": retrieval_sources_policy.structured_field_priority,
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

pub fn resolve_blend_policy(
	cfg: &RankingBlend,
	override_: Option<&BlendRankingOverride>,
) -> Result<ResolvedBlendPolicy> {
	let enabled = override_.and_then(|value| value.enabled).unwrap_or(cfg.enabled);
	let rerank_norm = override_
		.and_then(|value| value.rerank_normalization.as_deref())
		.unwrap_or(cfg.rerank_normalization.as_str());
	let retrieval_norm = override_
		.and_then(|value| value.retrieval_normalization.as_deref())
		.unwrap_or(cfg.retrieval_normalization.as_str());
	let rerank_normalization =
		parse_normalization_kind(rerank_norm, "ranking.blend.rerank_normalization")?;
	let retrieval_normalization =
		parse_normalization_kind(retrieval_norm, "ranking.blend.retrieval_normalization")?;
	let segments: Vec<BlendSegment> =
		if let Some(override_segments) = override_.and_then(|value| value.segments.as_ref()) {
			override_segments
				.iter()
				.map(|segment| BlendSegment {
					max_retrieval_rank: segment.max_retrieval_rank,
					retrieval_weight: segment.retrieval_weight,
				})
				.collect::<Vec<_>>()
		} else {
			cfg.segments
				.iter()
				.map(|segment| BlendSegment {
					max_retrieval_rank: segment.max_retrieval_rank,
					retrieval_weight: segment.retrieval_weight,
				})
				.collect::<Vec<_>>()
		};

	validate_blend_segments(&segments)?;

	Ok(ResolvedBlendPolicy { enabled, rerank_normalization, retrieval_normalization, segments })
}

pub fn resolve_diversity_policy(
	cfg: &RankingDiversity,
	override_: Option<&DiversityRankingOverride>,
) -> Result<ResolvedDiversityPolicy> {
	let enabled = override_.and_then(|value| value.enabled).unwrap_or(cfg.enabled);
	let sim_threshold =
		override_.and_then(|value| value.sim_threshold).unwrap_or(cfg.sim_threshold);
	let mmr_lambda = override_.and_then(|value| value.mmr_lambda).unwrap_or(cfg.mmr_lambda);
	let max_skips = override_.and_then(|value| value.max_skips).unwrap_or(cfg.max_skips);

	if !sim_threshold.is_finite() {
		return Err(Error::InvalidRequest {
			message: "ranking.diversity.sim_threshold must be a finite number.".to_string(),
		});
	}
	if !(0.0..=1.0).contains(&sim_threshold) {
		return Err(Error::InvalidRequest {
			message: "ranking.diversity.sim_threshold must be in the range 0.0-1.0.".to_string(),
		});
	}
	if !mmr_lambda.is_finite() {
		return Err(Error::InvalidRequest {
			message: "ranking.diversity.mmr_lambda must be a finite number.".to_string(),
		});
	}
	if !(0.0..=1.0).contains(&mmr_lambda) {
		return Err(Error::InvalidRequest {
			message: "ranking.diversity.mmr_lambda must be in the range 0.0-1.0.".to_string(),
		});
	}

	Ok(ResolvedDiversityPolicy { enabled, sim_threshold, mmr_lambda, max_skips })
}

pub fn resolve_retrieval_sources_policy(
	cfg: &RankingRetrievalSources,
	override_: Option<&RetrievalSourcesRankingOverride>,
) -> Result<ResolvedRetrievalSourcesPolicy> {
	let fusion_weight =
		override_.and_then(|value| value.fusion_weight).unwrap_or(cfg.fusion_weight);
	let structured_field_weight = override_
		.and_then(|value| value.structured_field_weight)
		.unwrap_or(cfg.structured_field_weight);
	let fusion_priority =
		override_.and_then(|value| value.fusion_priority).unwrap_or(cfg.fusion_priority);
	let structured_field_priority = override_
		.and_then(|value| value.structured_field_priority)
		.unwrap_or(cfg.structured_field_priority);

	for (path, value) in [
		("ranking.retrieval_sources.fusion_weight", fusion_weight),
		("ranking.retrieval_sources.structured_field_weight", structured_field_weight),
	] {
		if !value.is_finite() {
			return Err(Error::InvalidRequest {
				message: format!("{path} must be a finite number."),
			});
		}
		if value < 0.0 {
			return Err(Error::InvalidRequest {
				message: format!("{path} must be zero or greater."),
			});
		}
	}

	if fusion_weight <= 0.0 && structured_field_weight <= 0.0 {
		return Err(Error::InvalidRequest {
			message: "At least one retrieval source weight must be greater than zero.".to_string(),
		});
	}

	Ok(ResolvedRetrievalSourcesPolicy {
		fusion_weight,
		structured_field_weight,
		fusion_priority,
		structured_field_priority,
	})
}

pub fn parse_normalization_kind(value: &str, label: &str) -> Result<NormalizationKind> {
	match value.trim().to_ascii_lowercase().as_str() {
		"rank" => Ok(NormalizationKind::Rank),
		other => Err(Error::InvalidRequest {
			message: format!("{label} must be one of: rank. Got {other}."),
		}),
	}
}

pub fn validate_blend_segments(segments: &[BlendSegment]) -> Result<()> {
	if segments.is_empty() {
		return Err(Error::InvalidRequest {
			message: "ranking.blend.segments must be non-empty.".to_string(),
		});
	}

	let mut last_max = 0_u32;

	for (idx, segment) in segments.iter().enumerate() {
		if segment.max_retrieval_rank == 0 {
			return Err(Error::InvalidRequest {
				message: "ranking.blend.segments.max_retrieval_rank must be greater than zero."
					.to_string(),
			});
		}
		if idx > 0 && segment.max_retrieval_rank <= last_max {
			return Err(Error::InvalidRequest {
				message: "ranking.blend.segments.max_retrieval_rank must be strictly increasing."
					.to_string(),
			});
		}
		if !segment.retrieval_weight.is_finite() {
			return Err(Error::InvalidRequest {
				message: "ranking.blend.segments.retrieval_weight must be a finite number."
					.to_string(),
			});
		}
		if !(0.0..=1.0).contains(&segment.retrieval_weight) {
			return Err(Error::InvalidRequest {
				message: "ranking.blend.segments.retrieval_weight must be in the range 0.0-1.0."
					.to_string(),
			});
		}

		last_max = segment.max_retrieval_rank;
	}

	Ok(())
}

pub fn retrieval_weight_for_rank(rank: u32, segments: &[BlendSegment]) -> f32 {
	for segment in segments {
		if rank <= segment.max_retrieval_rank {
			return segment.retrieval_weight;
		}
	}

	segments.last().map(|segment| segment.retrieval_weight).unwrap_or(0.5)
}

pub fn resolve_scopes(cfg: &Config, profile: &str) -> Result<Vec<String>> {
	match profile {
		"private_only" => Ok(cfg.scopes.read_profiles.private_only.clone()),
		"private_plus_project" => Ok(cfg.scopes.read_profiles.private_plus_project.clone()),
		"all_scopes" => Ok(cfg.scopes.read_profiles.all_scopes.clone()),
		_ => Err(Error::InvalidRequest { message: "Unknown read_profile.".to_string() }),
	}
}
