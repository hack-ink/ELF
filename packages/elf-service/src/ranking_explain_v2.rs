use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use elf_config::Config;

pub const SEARCH_RANKING_EXPLAIN_SCHEMA_V2: &str = "search_ranking_explain/v2";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchRankingTerm {
	pub name: String,
	pub value: f32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub inputs: Option<BTreeMap<String, Value>>,
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
	let mut terms = Vec::new();
	let mut blend_retrieval_inputs = BTreeMap::new();

	blend_retrieval_inputs.insert("enabled".to_string(), serde_json::json!(blend_enabled));
	blend_retrieval_inputs
		.insert("retrieval_rank".to_string(), serde_json::json!(args.retrieval_rank));
	blend_retrieval_inputs
		.insert("retrieval_norm".to_string(), serde_json::json!(args.retrieval_norm));
	blend_retrieval_inputs.insert(
		"retrieval_normalization".to_string(),
		serde_json::json!(args.retrieval_normalization),
	);
	blend_retrieval_inputs.insert(
		"blend_retrieval_weight".to_string(),
		serde_json::json!(args.blend_retrieval_weight),
	);
	terms.push(SearchRankingTerm {
		name: "blend.retrieval".to_string(),
		value: args.retrieval_term,
		inputs: Some(blend_retrieval_inputs),
	});

	let mut blend_rerank_inputs = BTreeMap::new();

	blend_rerank_inputs.insert("enabled".to_string(), serde_json::json!(blend_enabled));
	blend_rerank_inputs.insert("rerank_score".to_string(), serde_json::json!(args.rerank_score));
	blend_rerank_inputs.insert("rerank_rank".to_string(), serde_json::json!(args.rerank_rank));
	blend_rerank_inputs.insert("rerank_norm".to_string(), serde_json::json!(args.rerank_norm));
	blend_rerank_inputs
		.insert("rerank_normalization".to_string(), serde_json::json!(args.rerank_normalization));
	blend_rerank_inputs.insert(
		"blend_retrieval_weight".to_string(),
		serde_json::json!(args.blend_retrieval_weight),
	);
	terms.push(SearchRankingTerm {
		name: "blend.rerank".to_string(),
		value: args.rerank_term,
		inputs: Some(blend_rerank_inputs),
	});

	let recency_decay = if cfg.ranking.recency_tau_days > 0.0 {
		(-args.age_days / cfg.ranking.recency_tau_days).exp()
	} else {
		1.0
	};
	let mut tie_breaker_inputs = BTreeMap::new();

	tie_breaker_inputs.insert(
		"tie_breaker_weight".to_string(),
		serde_json::json!(cfg.ranking.tie_breaker_weight),
	);
	tie_breaker_inputs.insert("importance".to_string(), serde_json::json!(args.importance));
	tie_breaker_inputs.insert("age_days".to_string(), serde_json::json!(args.age_days));
	tie_breaker_inputs
		.insert("recency_tau_days".to_string(), serde_json::json!(cfg.ranking.recency_tau_days));
	tie_breaker_inputs.insert("recency_decay".to_string(), serde_json::json!(recency_decay));
	terms.push(SearchRankingTerm {
		name: "tie_breaker".to_string(),
		value: args.tie_breaker_score,
		inputs: Some(tie_breaker_inputs),
	});

	let mut scope_boost_inputs = BTreeMap::new();

	scope_boost_inputs.insert("scope".to_string(), serde_json::json!(args.scope));
	scope_boost_inputs.insert(
		"scope_boost_weight".to_string(),
		serde_json::json!(cfg.context.as_ref().and_then(|ctx| ctx.scope_boost_weight)),
	);
	terms.push(SearchRankingTerm {
		name: "context.scope_boost".to_string(),
		value: args.scope_context_boost,
		inputs: Some(scope_boost_inputs),
	});

	let mut lex_inputs = BTreeMap::new();

	lex_inputs.insert("enabled".to_string(), serde_json::json!(det.enabled && det.lexical.enabled));
	lex_inputs.insert("weight".to_string(), serde_json::json!(det.lexical.weight));
	lex_inputs.insert("min_ratio".to_string(), serde_json::json!(det.lexical.min_ratio));
	lex_inputs
		.insert("max_query_terms".to_string(), serde_json::json!(det.lexical.max_query_terms));
	lex_inputs.insert("max_text_terms".to_string(), serde_json::json!(det.lexical.max_text_terms));
	lex_inputs.insert(
		"overlap_ratio".to_string(),
		serde_json::json!(args.deterministic_lexical_overlap_ratio),
	);
	terms.push(SearchRankingTerm {
		name: "deterministic.lexical_bonus".to_string(),
		value: args.deterministic_lexical_bonus,
		inputs: Some(lex_inputs),
	});

	let mut hits_inputs = BTreeMap::new();

	hits_inputs.insert("enabled".to_string(), serde_json::json!(det.enabled && det.hits.enabled));
	hits_inputs.insert("weight".to_string(), serde_json::json!(det.hits.weight));
	hits_inputs.insert("half_saturation".to_string(), serde_json::json!(det.hits.half_saturation));
	hits_inputs
		.insert("last_hit_tau_days".to_string(), serde_json::json!(det.hits.last_hit_tau_days));
	hits_inputs.insert("hit_count".to_string(), serde_json::json!(args.deterministic_hit_count));
	hits_inputs.insert(
		"last_hit_age_days".to_string(),
		serde_json::json!(args.deterministic_last_hit_age_days),
	);
	terms.push(SearchRankingTerm {
		name: "deterministic.hit_boost".to_string(),
		value: args.deterministic_hit_boost,
		inputs: Some(hits_inputs),
	});

	let mut decay_inputs = BTreeMap::new();

	decay_inputs.insert("enabled".to_string(), serde_json::json!(det.enabled && det.decay.enabled));
	decay_inputs.insert("weight".to_string(), serde_json::json!(det.decay.weight));
	decay_inputs.insert("tau_days".to_string(), serde_json::json!(det.decay.tau_days));
	decay_inputs.insert("age_days".to_string(), serde_json::json!(args.age_days));
	terms.push(SearchRankingTerm {
		name: "deterministic.decay_penalty".to_string(),
		value: args.deterministic_decay_penalty,
		inputs: Some(decay_inputs),
	});

	terms
}
