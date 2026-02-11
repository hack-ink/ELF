use std::collections::HashSet;

use serde_json::Value;

use crate::search::ExpansionMode;
use elf_config::{Config, SearchDynamic};
use elf_domain::cjk;

pub fn resolve_expansion_mode(cfg: &Config) -> ExpansionMode {
	match cfg.search.expansion.mode.as_str() {
		"off" => ExpansionMode::Off,
		"always" => ExpansionMode::Always,
		"dynamic" => ExpansionMode::Dynamic,
		_ => ExpansionMode::Off,
	}
}

pub fn should_expand_dynamic(candidate_count: usize, top_score: f32, cfg: &SearchDynamic) -> bool {
	candidate_count < cfg.min_candidates as usize || top_score < cfg.min_top_score
}

pub fn normalize_queries(
	queries: Vec<String>,
	original: &str,
	include_original: bool,
	max_queries: u32,
) -> Vec<String> {
	let mut out = Vec::new();
	let mut seen = HashSet::new();

	if include_original {
		push_query(&mut out, &mut seen, original);
	}

	for query in queries {
		if out.len() >= max_queries as usize {
			break;
		}

		push_query(&mut out, &mut seen, &query);
	}

	out.truncate(max_queries as usize);

	out
}

pub fn push_query(out: &mut Vec<String>, seen: &mut HashSet<String>, value: &str) {
	let trimmed = value.trim();

	if trimmed.is_empty() || cjk::contains_cjk(trimmed) {
		return;
	}

	let key = trimmed.to_lowercase();

	if seen.insert(key) {
		out.push(trimmed.to_string());
	}
}

pub fn build_expansion_messages(
	query: &str,
	max_queries: u32,
	include_original: bool,
) -> Vec<Value> {
	let schema = serde_json::json!({
		"queries": ["string"]
	});
	let schema_text = serde_json::to_string_pretty(&schema)
		.unwrap_or_else(|_| "{\"queries\": [\"string\"]}".to_string());
	let system_prompt = "You are a query expansion engine for a memory retrieval system. \
Output must be valid JSON only and must match the provided schema exactly. \
Generate short English-only query variations that preserve the original intent. \
Do not include any CJK characters. Do not add explanations or extra fields.";
	let user_prompt = format!(
		"Return JSON matching this exact schema:\n{schema}\nConstraints:\n- MAX_QUERIES = {max}\n- INCLUDE_ORIGINAL = {include}\nOriginal query:\n{query}",
		schema = schema_text,
		max = max_queries,
		include = include_original,
		query = query
	);

	vec![
		serde_json::json!({ "role": "system", "content": system_prompt }),
		serde_json::json!({ "role": "user", "content": user_prompt }),
	]
}

pub fn expansion_mode_label(mode: ExpansionMode) -> &'static str {
	match mode {
		ExpansionMode::Off => "off",
		ExpansionMode::Always => "always",
		ExpansionMode::Dynamic => "dynamic",
	}
}
