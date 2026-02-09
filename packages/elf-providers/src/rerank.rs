use std::{collections::HashSet, time::Duration};

use reqwest::Client;
use serde_json::Value;

use crate::{Error, Result};

pub async fn rerank(
	cfg: &elf_config::ProviderConfig,
	query: &str,
	docs: &[String],
) -> Result<Vec<f32>> {
	if cfg.provider_id == "local" {
		return Ok(local_rerank(query, docs));
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
		let scores = parse_rerank_response(json, 2).expect("parse failed");
		assert_eq!(scores, vec![0.9, 0.2]);
	}

	#[test]
	fn local_rerank_scores_match_token_overlap_fraction() {
		let scores = local_rerank("alpha beta", &[String::from("alpha"), String::from("gamma")]);
		assert_eq!(scores.len(), 2);
		assert!((scores[0] - 0.5).abs() < 1e-6, "Unexpected score: {}", scores[0]);
		assert_eq!(scores[1], 0.0);
	}
}
