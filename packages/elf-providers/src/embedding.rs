use std::time::Duration;

use color_eyre::{Result, eyre};
use reqwest::Client;
use serde_json::Value;

pub async fn embed(
	cfg: &elf_config::EmbeddingProviderConfig,
	texts: &[String],
) -> Result<Vec<Vec<f32>>> {
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

fn parse_embedding_response(json: Value) -> Result<Vec<Vec<f32>>> {
	let data = json
		.get("data")
		.and_then(|v| v.as_array())
		.ok_or_else(|| eyre::eyre!("Embedding response is missing data array."))?;

	let mut indexed: Vec<(usize, Vec<f32>)> = Vec::with_capacity(data.len());
	for (fallback_index, item) in data.iter().enumerate() {
		let index = item
			.get("index")
			.and_then(|v| v.as_u64())
			.map(|v| v as usize)
			.unwrap_or(fallback_index);
		let embedding = item
			.get("embedding")
			.and_then(|v| v.as_array())
			.ok_or_else(|| eyre::eyre!("Embedding item missing embedding array."))?;
		let mut vec = Vec::with_capacity(embedding.len());
		for value in embedding {
			let number =
				value.as_f64().ok_or_else(|| eyre::eyre!("Embedding value must be numeric."))?;
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
		let parsed = parse_embedding_response(json).expect("parse failed");
		assert_eq!(parsed.len(), 2);
		assert_eq!(parsed[0], vec![0.5, 1.5]);
		assert_eq!(parsed[1], vec![2.0, 3.0]);
	}
}
