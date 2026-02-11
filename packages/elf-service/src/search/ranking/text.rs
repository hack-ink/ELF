use std::collections::{HashMap, HashSet};

use time::OffsetDateTime;

use crate::search::DeterministicRankingTerms;
use elf_config::{Config, Context};
use elf_domain::cjk;

pub fn build_dense_embedding_input(
	query: &str,
	project_context_description: Option<&str>,
) -> String {
	let Some(description) = project_context_description else { return query.to_string() };
	let trimmed = description.trim();

	if trimmed.is_empty() {
		return query.to_string();
	}

	format!("{query}\n\nProject context:\n{trimmed}")
}

pub fn build_scope_context_boost_by_scope<'a>(
	tokens: &[String],
	context: Option<&'a Context>,
) -> HashMap<&'a str, f32> {
	let Some(context) = context else { return HashMap::new() };
	let Some(weight) = context.scope_boost_weight else { return HashMap::new() };

	if weight <= 0.0 || tokens.is_empty() {
		return HashMap::new();
	}

	let Some(descriptions) = context.scope_descriptions.as_ref() else { return HashMap::new() };
	let mut out = HashMap::new();

	for (scope, description) in descriptions {
		let boost = scope_description_boost(tokens, description, weight);

		if boost > 0.0 {
			out.insert(scope.as_str(), boost);
		}
	}

	out
}

pub fn scope_description_boost(tokens: &[String], description: &str, weight: f32) -> f32 {
	if weight <= 0.0 || tokens.is_empty() {
		return 0.0;
	}

	let trimmed = description.trim();

	if trimmed.is_empty() || cjk::contains_cjk(trimmed) {
		return 0.0;
	}

	let mut normalized = String::with_capacity(trimmed.len());

	for ch in trimmed.chars() {
		if ch.is_ascii_alphanumeric() {
			normalized.push(ch.to_ascii_lowercase());
		} else {
			normalized.push(' ');
		}
	}

	let mut description_tokens = HashSet::new();

	for token in normalized.split_whitespace() {
		if token.len() < 2 {
			continue;
		}

		description_tokens.insert(token);
	}

	if description_tokens.is_empty() {
		return 0.0;
	}

	let mut matched = 0_usize;

	for token in tokens {
		if description_tokens.contains(token.as_str()) {
			matched += 1;
		}
	}

	if matched == 0 {
		return 0.0;
	}

	weight * (matched as f32 / tokens.len() as f32)
}

pub fn tokenize_query(query: &str, max_terms: usize) -> Vec<String> {
	let mut normalized = String::with_capacity(query.len());

	for ch in query.chars() {
		if ch.is_ascii_alphanumeric() {
			normalized.push(ch.to_ascii_lowercase());
		} else {
			normalized.push(' ');
		}
	}

	let mut out = Vec::new();
	let mut seen = HashSet::new();

	for token in normalized.split_whitespace() {
		if token.len() < 2 {
			continue;
		}
		if seen.insert(token) {
			out.push(token.to_string());
		}
		if out.len() >= max_terms {
			break;
		}
	}

	out
}

pub fn tokenize_text_terms(text: &str, max_terms: usize) -> HashSet<String> {
	if max_terms == 0 {
		return HashSet::new();
	}

	let mut normalized = String::with_capacity(text.len());

	for ch in text.chars() {
		if ch.is_ascii_alphanumeric() {
			normalized.push(ch.to_ascii_lowercase());
		} else {
			normalized.push(' ');
		}
	}

	let mut out = HashSet::new();

	for token in normalized.split_whitespace() {
		if token.len() < 2 {
			continue;
		}

		out.insert(token.to_string());

		if out.len() >= max_terms {
			break;
		}
	}

	out
}

pub fn lexical_overlap_ratio(query_tokens: &[String], text: &str, max_text_terms: usize) -> f32 {
	if query_tokens.is_empty() {
		return 0.0;
	}

	let text_terms = tokenize_text_terms(text, max_text_terms);

	if text_terms.is_empty() {
		return 0.0;
	}

	let mut matched = 0_usize;

	for token in query_tokens {
		if text_terms.contains(token.as_str()) {
			matched += 1;
		}
	}

	matched as f32 / query_tokens.len() as f32
}

pub fn compute_deterministic_ranking_terms(
	cfg: &Config,
	query_tokens: &[String],
	snippet: &str,
	note_hit_count: i64,
	note_last_hit_at: Option<OffsetDateTime>,
	age_days: f32,
	now: OffsetDateTime,
) -> DeterministicRankingTerms {
	let det = &cfg.ranking.deterministic;

	if !det.enabled {
		return DeterministicRankingTerms::default();
	}

	let mut out = DeterministicRankingTerms::default();

	if det.lexical.enabled && det.lexical.weight > 0.0 && !query_tokens.is_empty() {
		let ratio =
			lexical_overlap_ratio(query_tokens, snippet, det.lexical.max_text_terms as usize);

		out.lexical_overlap_ratio = ratio;

		let min_ratio = det.lexical.min_ratio.clamp(0.0, 1.0);
		let scaled = if ratio >= min_ratio && min_ratio < 1.0 {
			((ratio - min_ratio) / (1.0 - min_ratio)).clamp(0.0, 1.0)
		} else if ratio >= 1.0 && min_ratio >= 1.0 {
			1.0
		} else {
			0.0
		};

		out.lexical_bonus = det.lexical.weight * scaled;
	}
	if det.hits.enabled && det.hits.weight > 0.0 {
		let hit_count = note_hit_count.max(0);

		out.hit_count = hit_count;

		let half = det.hits.half_saturation;
		let hit_saturation = if half > 0.0 && hit_count > 0 {
			let hc = hit_count as f32;

			(hc / (hc + half)).clamp(0.0, 1.0)
		} else {
			0.0
		};
		let last_hit_age_days =
			note_last_hit_at.map(|ts| ((now - ts).as_seconds_f32() / 86_400.0).max(0.0));

		out.last_hit_age_days = last_hit_age_days;

		let tau = det.hits.last_hit_tau_days;
		let recency = if tau > 0.0 {
			match last_hit_age_days {
				Some(days) => (-days / tau).exp(),
				None => 1.0,
			}
		} else {
			1.0
		};

		out.hit_boost = det.hits.weight * hit_saturation * recency;
	}
	if det.decay.enabled && det.decay.weight > 0.0 {
		let age_days = age_days.max(0.0);
		let tau = det.decay.tau_days;
		let staleness = if tau > 0.0 { 1.0 - (-age_days / tau).exp() } else { 0.0 };

		out.decay_penalty = -det.decay.weight * staleness.clamp(0.0, 1.0);
	}

	out
}

pub fn match_terms_in_text(
	tokens: &[String],
	text: &str,
	key: Option<&str>,
	max_terms: usize,
) -> (Vec<String>, Vec<String>) {
	if tokens.is_empty() {
		return (Vec::new(), Vec::new());
	}

	let text = text.to_lowercase();
	let key = key.map(|value| value.to_lowercase());
	let mut matched_terms = Vec::new();
	let mut matched_fields = HashSet::new();

	for token in tokens {
		let mut matched = false;

		if text.contains(token) {
			matched_fields.insert("text");

			matched = true;
		}

		if let Some(key) = key.as_ref()
			&& key.contains(token)
		{
			matched_fields.insert("key");

			matched = true;
		}

		if matched {
			matched_terms.push(token.clone());
		}
		if matched_terms.len() >= max_terms {
			break;
		}
	}

	let mut fields: Vec<String> =
		matched_fields.into_iter().map(|field| field.to_string()).collect();

	fields.sort();

	(matched_terms, fields)
}

pub fn merge_matched_fields(mut base: Vec<String>, extra: Option<&Vec<String>>) -> Vec<String> {
	if let Some(extra) = extra {
		for field in extra {
			base.push(field.clone());
		}

		base.sort();
		base.dedup();
	}

	base
}
