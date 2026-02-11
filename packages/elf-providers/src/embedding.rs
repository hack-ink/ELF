use std::time::Duration;

use reqwest::Client;
use serde_json::Value;

use crate::{Error, Result};

pub async fn embed(
	cfg: &elf_config::EmbeddingProviderConfig,
	texts: &[String],
) -> Result<Vec<Vec<f32>>> {
	if cfg.provider_id == "local" {
		let dim = cfg.dimensions as usize;

		return Ok(texts.iter().map(|text| local_embed(dim, text)).collect());
	}

	let client = Client::builder().timeout(Duration::from_millis(cfg.timeout_ms)).build()?;
	let url = format!("{}{}", cfg.api_base, cfg.path);
	let body = serde_json::json!({
		"model": cfg.model,
		"input": texts,
		"dimensions": cfg.dimensions,
	});
	let res = client
		.post(url)
		.headers(crate::auth_headers(&cfg.api_key, &cfg.default_headers)?)
		.json(&body)
		.send()
		.await?;
	let json: Value = res.error_for_status()?.json().await?;

	parse_embedding_response(json)
}

fn local_embed(dim: usize, text: &str) -> Vec<f32> {
	let mut vec = vec![0.0_f32; dim];

	if dim == 0 {
		return vec;
	}

	let normalized = normalize_ascii_alnum_lowercase(text);

	for token in normalized.split_whitespace() {
		if token.len() < 2 {
			continue;
		}

		let hash = blake3::hash(token.as_bytes());
		let bytes = hash.as_bytes();
		let index = (u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize) % dim;
		let sign = if bytes[4] & 1 == 0 { 1.0 } else { -1.0 };

		vec[index] += sign;
	}

	if vec.iter().all(|value| *value == 0.0) {
		let hash = blake3::hash(text.as_bytes());
		let bytes = hash.as_bytes();
		let index = (u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize) % dim;

		vec[index] = 1.0;
	}

	l2_normalize(&mut vec);

	vec
}

fn normalize_ascii_alnum_lowercase(text: &str) -> String {
	let mut normalized = String::with_capacity(text.len());

	for ch in text.chars() {
		if ch.is_ascii_alphanumeric() {
			normalized.push(ch.to_ascii_lowercase());
		} else {
			normalized.push(' ');
		}
	}

	normalized
}

fn l2_normalize(vec: &mut [f32]) {
	let mut norm = 0.0_f32;

	for value in vec.iter() {
		norm += value * value;
	}

	if norm <= 0.0 {
		return;
	}

	let inv = 1.0 / norm.sqrt();

	for value in vec.iter_mut() {
		*value *= inv;
	}
}

fn parse_embedding_response(json: Value) -> Result<Vec<Vec<f32>>> {
	let data = json.get("data").and_then(|v| v.as_array()).ok_or_else(|| {
		Error::InvalidResponse { message: "Embedding response is missing data array.".to_string() }
	})?;
	let mut indexed: Vec<(usize, Vec<f32>)> = Vec::with_capacity(data.len());

	for (fallback_index, item) in data.iter().enumerate() {
		let index = item
			.get("index")
			.and_then(|v| v.as_u64())
			.map(|v| v as usize)
			.unwrap_or(fallback_index);
		let embedding = item.get("embedding").and_then(|v| v.as_array()).ok_or_else(|| {
			Error::InvalidResponse {
				message: "Embedding item missing embedding array.".to_string(),
			}
		})?;
		let mut vec = Vec::with_capacity(embedding.len());

		for value in embedding {
			let number = value.as_f64().ok_or_else(|| Error::InvalidResponse {
				message: "Embedding value must be numeric.".to_string(),
			})?;

			vec.push(number as f32);
		}

		indexed.push((index, vec));
	}

	indexed.sort_by_key(|(index, _)| *index);

	Ok(indexed.into_iter().map(|(_, vec)| vec).collect())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_embeddings_in_index_order() {
		let json = serde_json::json!({
			"data": [
				{ "index": 1, "embedding": [2.0, 3.0] },
				{ "index": 0, "embedding": [0.5, 1.5] }
			]
		});
		let parsed = parse_embedding_response(json).expect("Parsing should succeed.");

		assert_eq!(parsed.len(), 2);
		assert_eq!(parsed[0], vec![0.5, 1.5]);
		assert_eq!(parsed[1], vec![2.0, 3.0]);
	}

	#[test]
	fn local_embedding_is_deterministic_and_has_expected_dimension() {
		let a = local_embed(64, "Embeddings are stored in Postgres.");
		let b = local_embed(64, "Embeddings are stored in Postgres.");

		assert_eq!(a.len(), 64);
		assert_eq!(a, b);
	}

	#[test]
	fn local_embedding_is_more_similar_for_shared_tokens() {
		let a = local_embed(512, "alpha beta");
		let b = local_embed(512, "alpha gamma");
		let c = local_embed(512, "delta epsilon");
		let sim_ab = dot(&a, &b);
		let sim_ac = dot(&a, &c);

		assert!(
			sim_ab > sim_ac,
			"Expected shared-token similarity to be higher. sim_ab={sim_ab}, sim_ac={sim_ac}"
		);
	}

	fn dot(a: &[f32], b: &[f32]) -> f32 {
		a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
	}
}
