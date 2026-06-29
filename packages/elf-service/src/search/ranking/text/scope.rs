use std::collections::{HashMap, HashSet};

use elf_config::Context;
use elf_domain::english_gate;

pub(super) fn build_scope_context_boost_by_scope<'a>(
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

fn scope_description_boost(tokens: &[String], description: &str, weight: f32) -> f32 {
	if weight <= 0.0 || tokens.is_empty() {
		return 0.0;
	}

	let trimmed = description.trim();

	if trimmed.is_empty() || !english_gate::is_english_natural_language(trimmed) {
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

#[cfg(test)]
mod tests {
	use crate::search::ranking::text::scope;

	#[test]
	fn scope_description_boost_matches_whole_tokens_only() {
		let tokens = vec!["go".to_string()];
		let boost = scope::scope_description_boost(&tokens, "MongoDB operational notes.", 0.1);

		assert_eq!(boost, 0.0);
	}

	#[test]
	fn scope_description_boost_scales_by_fraction_of_matched_tokens() {
		let tokens = vec!["security".to_string(), "policy".to_string(), "deployment".to_string()];
		let boost = scope::scope_description_boost(&tokens, "Security policy notes.", 0.12);

		assert!((boost - 0.08).abs() < 1e-4, "Unexpected boost: {boost}");
	}
}
