use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use elf_config::Config;

pub const SEARCH_RANKING_EXPLAIN_SCHEMA_V2: &str = "search_ranking_explain/v2";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchRankingTerm {
	pub name: String,
	pub value: f32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub inputs: Option<BTreeMap<String, serde_json::Value>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchRankingExplain {
	pub schema: String,
	pub policy_id: String,
	pub final_score: f32,
	pub terms: Vec<SearchRankingTerm>,
}

pub struct TraceTermsArgs<'a> {
	pub cfg: &'a Config,
	pub blend_enabled: bool,
	pub retrieval_normalization: &'a str,
	pub rerank_normalization: &'a str,
	pub blend_retrieval_weight: f32,
	pub retrieval_rank: u32,
	pub retrieval_norm: f32,
	pub retrieval_term: f32,
	pub rerank_score: f32,
	pub rerank_rank: u32,
	pub rerank_norm: f32,
	pub rerank_term: f32,
	pub tie_breaker_score: f32,
	pub importance: f32,
	pub age_days: f32,
	pub scope: &'a str,
	pub scope_context_boost: f32,
	pub deterministic_lexical_overlap_ratio: f32,
	pub deterministic_lexical_bonus: f32,
	pub deterministic_hit_count: i64,
	pub deterministic_last_hit_age_days: Option<f32>,
	pub deterministic_hit_boost: f32,
	pub deterministic_decay_penalty: f32,
}

pub fn strip_term_inputs(terms: &[SearchRankingTerm]) -> Vec<SearchRankingTerm> {
	terms
		.iter()
		.map(|term| SearchRankingTerm { name: term.name.clone(), value: term.value, inputs: None })
		.collect()
}

pub fn build_trace_terms_v2(args: TraceTermsArgs<'_>) -> Vec<SearchRankingTerm> {
	let cfg = args.cfg;
	let blend_enabled = args.blend_enabled;
	let det = &cfg.ranking.deterministic;

	vec![
		build_blend_retrieval_term(&args, blend_enabled),
		build_blend_rerank_term(&args, blend_enabled),
		build_tie_breaker_term(&args, cfg),
		build_scope_boost_term(&args, cfg),
		build_deterministic_lexical_term(&args, det),
		build_deterministic_hit_term(&args, det),
		build_deterministic_decay_term(&args, det),
	]
}

fn build_blend_retrieval_term(args: &TraceTermsArgs<'_>, blend_enabled: bool) -> SearchRankingTerm {
	let mut inputs = BTreeMap::new();

	inputs.insert("enabled".to_string(), serde_json::json!(blend_enabled));
	inputs.insert("retrieval_rank".to_string(), serde_json::json!(args.retrieval_rank));
	inputs.insert("retrieval_norm".to_string(), serde_json::json!(args.retrieval_norm));
	inputs.insert(
		"retrieval_normalization".to_string(),
		serde_json::json!(args.retrieval_normalization),
	);
	inputs.insert(
		"blend_retrieval_weight".to_string(),
		serde_json::json!(args.blend_retrieval_weight),
	);
	SearchRankingTerm {
		name: "blend.retrieval".to_string(),
		value: args.retrieval_term,
		inputs: Some(inputs),
	}
}

fn build_blend_rerank_term(args: &TraceTermsArgs<'_>, blend_enabled: bool) -> SearchRankingTerm {
	let mut inputs = BTreeMap::new();

	inputs.insert("enabled".to_string(), serde_json::json!(blend_enabled));
	inputs.insert("rerank_score".to_string(), serde_json::json!(args.rerank_score));
	inputs.insert("rerank_rank".to_string(), serde_json::json!(args.rerank_rank));
	inputs.insert("rerank_norm".to_string(), serde_json::json!(args.rerank_norm));
	inputs.insert("rerank_normalization".to_string(), serde_json::json!(args.rerank_normalization));
	inputs.insert(
		"blend_retrieval_weight".to_string(),
		serde_json::json!(args.blend_retrieval_weight),
	);
	SearchRankingTerm {
		name: "blend.rerank".to_string(),
		value: args.rerank_term,
		inputs: Some(inputs),
	}
}

fn build_tie_breaker_term(args: &TraceTermsArgs<'_>, cfg: &Config) -> SearchRankingTerm {
	let recency_decay = if cfg.ranking.recency_tau_days > 0.0 {
		(-args.age_days / cfg.ranking.recency_tau_days).exp()
	} else {
		1.0
	};
	let mut inputs = BTreeMap::new();

	inputs.insert(
		"tie_breaker_weight".to_string(),
		serde_json::json!(cfg.ranking.tie_breaker_weight),
	);
	inputs.insert("importance".to_string(), serde_json::json!(args.importance));
	inputs.insert("age_days".to_string(), serde_json::json!(args.age_days));
	inputs.insert("recency_tau_days".to_string(), serde_json::json!(cfg.ranking.recency_tau_days));
	inputs.insert("recency_decay".to_string(), serde_json::json!(recency_decay));
	SearchRankingTerm {
		name: "tie_breaker".to_string(),
		value: args.tie_breaker_score,
		inputs: Some(inputs),
	}
}

fn build_scope_boost_term(args: &TraceTermsArgs<'_>, cfg: &Config) -> SearchRankingTerm {
	let mut inputs = BTreeMap::new();

	inputs.insert("scope".to_string(), serde_json::json!(args.scope));
	inputs.insert(
		"scope_boost_weight".to_string(),
		serde_json::json!(cfg.context.as_ref().and_then(|ctx| ctx.scope_boost_weight)),
	);
	SearchRankingTerm {
		name: "context.scope_boost".to_string(),
		value: args.scope_context_boost,
		inputs: Some(inputs),
	}
}

fn build_deterministic_lexical_term(
	args: &TraceTermsArgs<'_>,
	det: &elf_config::RankingDeterministic,
) -> SearchRankingTerm {
	let mut inputs = BTreeMap::new();

	inputs.insert("enabled".to_string(), serde_json::json!(det.enabled && det.lexical.enabled));
	inputs.insert("weight".to_string(), serde_json::json!(det.lexical.weight));
	inputs.insert("min_ratio".to_string(), serde_json::json!(det.lexical.min_ratio));
	inputs.insert("max_query_terms".to_string(), serde_json::json!(det.lexical.max_query_terms));
	inputs.insert("max_text_terms".to_string(), serde_json::json!(det.lexical.max_text_terms));
	inputs.insert(
		"overlap_ratio".to_string(),
		serde_json::json!(args.deterministic_lexical_overlap_ratio),
	);
	SearchRankingTerm {
		name: "deterministic.lexical_bonus".to_string(),
		value: args.deterministic_lexical_bonus,
		inputs: Some(inputs),
	}
}

fn build_deterministic_hit_term(
	args: &TraceTermsArgs<'_>,
	det: &elf_config::RankingDeterministic,
) -> SearchRankingTerm {
	let mut inputs = BTreeMap::new();

	inputs.insert("enabled".to_string(), serde_json::json!(det.enabled && det.hits.enabled));
	inputs.insert("weight".to_string(), serde_json::json!(det.hits.weight));
	inputs.insert("half_saturation".to_string(), serde_json::json!(det.hits.half_saturation));
	inputs.insert("last_hit_tau_days".to_string(), serde_json::json!(det.hits.last_hit_tau_days));
	inputs.insert("hit_count".to_string(), serde_json::json!(args.deterministic_hit_count));
	inputs.insert(
		"last_hit_age_days".to_string(),
		serde_json::json!(args.deterministic_last_hit_age_days),
	);
	SearchRankingTerm {
		name: "deterministic.hit_boost".to_string(),
		value: args.deterministic_hit_boost,
		inputs: Some(inputs),
	}
}

fn build_deterministic_decay_term(
	args: &TraceTermsArgs<'_>,
	det: &elf_config::RankingDeterministic,
) -> SearchRankingTerm {
	let mut inputs = BTreeMap::new();

	inputs.insert("enabled".to_string(), serde_json::json!(det.enabled && det.decay.enabled));
	inputs.insert("weight".to_string(), serde_json::json!(det.decay.weight));
	inputs.insert("tau_days".to_string(), serde_json::json!(det.decay.tau_days));
	inputs.insert("age_days".to_string(), serde_json::json!(args.age_days));
	SearchRankingTerm {
		name: "deterministic.decay_penalty".to_string(),
		value: args.deterministic_decay_penalty,
		inputs: Some(inputs),
	}
}
