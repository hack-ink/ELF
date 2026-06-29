use crate::{
	Error, Result,
	search::{
		BlendRankingOverride, DiversityRankingOverride, RetrievalSourcesRankingOverride,
		ranking::policy::types::{
			BlendSegment, NormalizationKind, ResolvedBlendPolicy, ResolvedDiversityPolicy,
			ResolvedRetrievalSourcesPolicy,
		},
	},
};
use elf_config::{Config, RankingBlend, RankingDiversity, RankingRetrievalSources};

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
	let recursive_weight =
		override_.and_then(|value| value.recursive_weight).unwrap_or(structured_field_weight);
	let fusion_priority =
		override_.and_then(|value| value.fusion_priority).unwrap_or(cfg.fusion_priority);
	let structured_field_priority = override_
		.and_then(|value| value.structured_field_priority)
		.unwrap_or(cfg.structured_field_priority);
	let recursive_priority = override_
		.and_then(|value| value.recursive_priority)
		.unwrap_or(structured_field_priority.saturating_add(1));

	for (path, value) in [
		("ranking.retrieval_sources.fusion_weight", fusion_weight),
		("ranking.retrieval_sources.structured_field_weight", structured_field_weight),
		("ranking.retrieval_sources.recursive_weight", recursive_weight),
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

	if fusion_weight <= 0.0 && structured_field_weight <= 0.0 && recursive_weight <= 0.0 {
		return Err(Error::InvalidRequest {
			message: "At least one retrieval source weight must be greater than zero.".to_string(),
		});
	}

	Ok(ResolvedRetrievalSourcesPolicy {
		fusion_weight,
		structured_field_weight,
		recursive_weight,
		fusion_priority,
		structured_field_priority,
		recursive_priority,
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
