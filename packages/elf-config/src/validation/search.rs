use crate::{Config, Error, Result};

pub(super) fn validate(cfg: &Config) -> Result<()> {
	validate_expansion(cfg)?;
	validate_dynamic(cfg)?;
	validate_cache(cfg)?;
	validate_explain(cfg)?;
	validate_explain_write_mode(cfg)?;
	validate_recursive(cfg)?;

	Ok(())
}

fn validate_expansion(cfg: &Config) -> Result<()> {
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

	Ok(())
}

fn validate_dynamic(cfg: &Config) -> Result<()> {
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

	Ok(())
}

fn validate_cache(cfg: &Config) -> Result<()> {
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

	Ok(())
}

fn validate_explain(cfg: &Config) -> Result<()> {
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

	Ok(())
}

fn validate_explain_write_mode(cfg: &Config) -> Result<()> {
	match cfg.search.explain.write_mode.trim().to_ascii_lowercase().as_str() {
		"outbox" | "inline" => Ok(()),
		other => Err(Error::Validation {
			message: format!(
				"search.explain.write_mode must be one of: outbox, inline. Got {other}."
			),
		}),
	}
}

fn validate_recursive(cfg: &Config) -> Result<()> {
	if !cfg.search.recursive.enabled {
		return Ok(());
	}
	if cfg.search.recursive.max_depth == 0 {
		return Err(Error::Validation {
			message: "search.recursive.max_depth must be greater than zero.".to_string(),
		});
	}
	if cfg.search.recursive.max_depth > 8 {
		return Err(Error::Validation {
			message: "search.recursive.max_depth must be 8 or less.".to_string(),
		});
	}
	if cfg.search.recursive.max_children_per_node == 0 {
		return Err(Error::Validation {
			message: "search.recursive.max_children_per_node must be greater than zero."
				.to_string(),
		});
	}
	if cfg.search.recursive.max_children_per_node > 64 {
		return Err(Error::Validation {
			message: "search.recursive.max_children_per_node must be 64 or less.".to_string(),
		});
	}
	if cfg.search.recursive.max_nodes_per_scope == 0 {
		return Err(Error::Validation {
			message: "search.recursive.max_nodes_per_scope must be greater than zero.".to_string(),
		});
	}
	if cfg.search.recursive.max_nodes_per_scope > 250 {
		return Err(Error::Validation {
			message: "search.recursive.max_nodes_per_scope must be 250 or less.".to_string(),
		});
	}
	if cfg.search.recursive.max_total_nodes == 0 {
		return Err(Error::Validation {
			message: "search.recursive.max_total_nodes must be greater than zero.".to_string(),
		});
	}
	if cfg.search.recursive.max_total_nodes > 2_000 {
		return Err(Error::Validation {
			message: "search.recursive.max_total_nodes must be 2_000 or less.".to_string(),
		});
	}
	if cfg.search.recursive.max_total_nodes < cfg.search.recursive.max_nodes_per_scope {
		return Err(Error::Validation {
			message:
				"search.recursive.max_total_nodes must be at least search.recursive.max_nodes_per_scope."
					.to_string(),
		});
	}

	Ok(())
}

pub(super) fn validate_graph_context(cfg: &Config) -> Result<()> {
	if !cfg.search.graph_context.enabled {
		return Ok(());
	}

	let ctx = &cfg.search.graph_context;

	if ctx.max_facts_per_item == 0 {
		return Err(Error::Validation {
			message: "search.graph_context.max_facts_per_item must be greater than zero."
				.to_string(),
		});
	}
	if ctx.max_facts_per_item > 1_000 {
		return Err(Error::Validation {
			message: "search.graph_context.max_facts_per_item must be 1,000 or less.".to_string(),
		});
	}
	if ctx.max_evidence_notes_per_fact == 0 {
		return Err(Error::Validation {
			message: "search.graph_context.max_evidence_notes_per_fact must be greater than zero."
				.to_string(),
		});
	}
	if ctx.max_evidence_notes_per_fact > 1_000 {
		return Err(Error::Validation {
			message: "search.graph_context.max_evidence_notes_per_fact must be 1,000 or less."
				.to_string(),
		});
	}

	Ok(())
}
