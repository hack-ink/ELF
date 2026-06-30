use std::collections::HashSet;

use crate::rerank::noise;

pub(super) fn local_rerank_dispatch(model: &str, query: &str, docs: &[String]) -> Vec<f32> {
	if let Some(noise_std) = parse_local_noisy_model(model) {
		return local_rerank_noisy(query, docs, noise_std);
	}

	local_rerank(query, docs)
}

pub(super) fn parse_local_noisy_model(model: &str) -> Option<f32> {
	let prefix = "local-token-overlap-noisy@";
	let rest = model.strip_prefix(prefix)?;
	let std: f32 = rest.parse().ok()?;

	Some(std.max(0.0))
}

pub(super) fn local_rerank(query: &str, docs: &[String]) -> Vec<f32> {
	let query_tokens = tokenize_ascii_alnum(query);

	if query_tokens.is_empty() {
		return vec![0.0; docs.len()];
	}

	let denom = query_tokens.len() as f32;
	let mut scores = Vec::with_capacity(docs.len());

	for doc in docs {
		let doc_tokens = tokenize_ascii_alnum(doc);
		let matched = query_tokens.intersection(&doc_tokens).count() as f32;

		scores.push(matched / denom);
	}

	scores
}

fn local_rerank_noisy(query: &str, docs: &[String], noise_std: f32) -> Vec<f32> {
	let base = local_rerank(query, docs);

	if noise_std <= 0.0 {
		return base;
	}

	let seed = noise::seed_for_query_call(query);
	let mut out = Vec::with_capacity(base.len());

	for (i, score) in base.into_iter().enumerate() {
		let signed = noise::signed_unit_noise(seed, i);
		let noisy = score + signed * noise_std;

		out.push(noisy.clamp(0.0, 1.0));
	}

	out
}

fn tokenize_ascii_alnum(text: &str) -> HashSet<String> {
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
	}

	out
}
