use crate::{Config, Error, Result};

pub(super) fn validate(cfg: &Config) -> Result<()> {
	validate_core(cfg)?;
	validate_blend(cfg)?;
	validate_diversity(cfg)?;
	validate_retrieval_sources(cfg)?;
	validate_deterministic(cfg)?;

	Ok(())
}

fn validate_core(cfg: &Config) -> Result<()> {
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

	Ok(())
}

fn validate_blend(cfg: &Config) -> Result<()> {
	if !cfg.ranking.blend.enabled {
		return Ok(());
	}
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
				message: "ranking.blend.segments.retrieval_weight must be in the range 0.0-1.0."
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

	Ok(())
}

fn validate_diversity(cfg: &Config) -> Result<()> {
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

fn validate_retrieval_sources(cfg: &Config) -> Result<()> {
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

fn validate_deterministic(cfg: &Config) -> Result<()> {
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
