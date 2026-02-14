use std::{collections::HashSet, time::Duration};

use reqwest::Client;
use serde_json::Value;

use crate::{Error, Result};

static LOCAL_NOISE_CALL_COUNTER: std::sync::atomic::AtomicU64 =
	std::sync::atomic::AtomicU64::new(0);

struct XorShift64 {
	state: u64,
}
impl XorShift64 {
	fn new(seed: u64) -> Self {
		let state = if seed == 0 { 0x4D59_5DF4_D0F3_3173 } else { seed };

		Self { state }
	}

	fn next_u64(&mut self) -> u64 {
		let mut x = self.state;
		x ^= x << 13;
		x ^= x >> 7;
		x ^= x << 17;
		self.state = x;

		x
	}

	fn next_f32(&mut self) -> f32 {
		// Map to [0, 1). Keep 24 bits of precision for a stable f32.
		let bits = (self.next_u64() >> 40) as u32;

		(bits as f32) / ((1u32 << 24) as f32)
	}
}

pub async fn rerank(
	cfg: &elf_config::ProviderConfig,
	query: &str,
	docs: &[String],
) -> Result<Vec<f32>> {
	if cfg.provider_id == "local" {
		return Ok(local_rerank_dispatch(cfg.model.as_str(), query, docs));
	}

	let client = Client::builder().timeout(Duration::from_millis(cfg.timeout_ms)).build()?;
	let url = format!("{}{}", cfg.api_base, cfg.path);
	let body = serde_json::json!({ "model": cfg.model, "query": query, "documents": docs });
	let res = client
		.post(url)
		.headers(crate::auth_headers(&cfg.api_key, &cfg.default_headers)?)
		.json(&body)
		.send()
		.await?;
	let json: Value = res.error_for_status()?.json().await?;

	parse_rerank_response(json, docs.len())
}

fn local_rerank_dispatch(model: &str, query: &str, docs: &[String]) -> Vec<f32> {
	if let Some(noise_std) = parse_local_noisy_model(model) {
		return local_rerank_noisy(query, docs, noise_std);
	}

	local_rerank(query, docs)
}

fn parse_local_noisy_model(model: &str) -> Option<f32> {
	let prefix = "local-token-overlap-noisy@";
	let rest = model.strip_prefix(prefix)?;
	let std: f32 = rest.parse().ok()?;

	Some(std.max(0.0))
}

fn local_rerank(query: &str, docs: &[String]) -> Vec<f32> {
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

	let query_hash = blake3::hash(query.as_bytes());
	let mut seed_bytes = [0_u8; 8];

	seed_bytes.copy_from_slice(&query_hash.as_bytes()[..8]);
	// Vary the noise across calls to simulate reranker instability.

	let call_idx = LOCAL_NOISE_CALL_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	let mut seed = u64::from_le_bytes(seed_bytes);

	seed ^= call_idx.wrapping_mul(0x9E37_79B9_7F4A_7C15);

	let mut out = Vec::with_capacity(base.len());

	for (i, score) in base.into_iter().enumerate() {
		let mut rng = XorShift64::new(seed ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
		let u = rng.next_f32();
		let signed = (u * 2.0) - 1.0;
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

fn parse_rerank_response(json: Value, doc_count: usize) -> Result<Vec<f32>> {
	let mut scores = vec![0.0f32; doc_count];
	let results =
		json.get("results").or_else(|| json.get("data")).and_then(|v| v.as_array()).ok_or_else(
			|| Error::InvalidResponse {
				message: "Rerank response is missing results array.".to_string(),
			},
		)?;

	for item in results {
		let index = item.get("index").and_then(|v| v.as_u64()).ok_or_else(|| {
			Error::InvalidResponse { message: "Rerank result missing index.".to_string() }
		})? as usize;
		let score = item
			.get("relevance_score")
			.or_else(|| item.get("score"))
			.and_then(|v| v.as_f64())
			.ok_or_else(|| Error::InvalidResponse {
				message: "Rerank result missing score.".to_string(),
			})? as f32;

		if index < scores.len() {
			scores[index] = score;
		}
	}

	Ok(scores)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn aligns_scores_by_index() {
		let json = serde_json::json!({
			"results": [
				{ "index": 1, "relevance_score": 0.2 },
				{ "index": 0, "relevance_score": 0.9 }
			]
		});
		let scores = parse_rerank_response(json, 2)
			.expect("Rerank response parsing must succeed for the valid JSON fixture.");

		assert_eq!(scores, vec![0.9, 0.2]);
	}

	#[test]
	fn local_rerank_scores_match_token_overlap_fraction() {
		let scores = local_rerank("alpha beta", &[String::from("alpha"), String::from("gamma")]);

		assert_eq!(scores.len(), 2);
		assert!((scores[0] - 0.5).abs() < 1e-6, "Unexpected score: {}", scores[0]);
		assert_eq!(scores[1], 0.0);
	}

	#[test]
	fn local_noisy_model_is_detected_and_nonnegative() {
		assert_eq!(parse_local_noisy_model("local-token-overlap"), None);
		assert_eq!(parse_local_noisy_model("local-token-overlap-noisy@0.02"), Some(0.02));
		assert_eq!(parse_local_noisy_model("local-token-overlap-noisy@-1"), Some(0.0));
	}

	#[test]
	fn local_rerank_noisy_varies_across_calls() {
		// Use a base score away from 0 and 1 so clamping does not mask noise.
		let docs = [String::from("alpha"), String::from("alpha")];
		let first = local_rerank_dispatch("local-token-overlap-noisy@0.1", "alpha beta", &docs);

		assert!(first.iter().all(|v| (0.0..=1.0).contains(v)));

		let mut varied = false;

		for _ in 0..32 {
			let next = local_rerank_dispatch("local-token-overlap-noisy@0.1", "alpha beta", &docs);

			assert_eq!(first.len(), next.len());
			assert!(next.iter().all(|v| (0.0..=1.0).contains(v)));

			if next != first {
				varied = true;

				break;
			}
		}

		assert!(varied, "Expected noisy rerank to vary across calls.");
	}
}
