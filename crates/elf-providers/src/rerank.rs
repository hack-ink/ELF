use color_eyre::{Result, eyre::eyre};

use crate::auth_headers;

pub async fn rerank(
	cfg: &elf_config::ProviderConfig,
	query: &str,
	docs: &[String],
) -> Result<Vec<f32>> {
	let client = reqwest::Client::builder()
		.timeout(std::time::Duration::from_millis(cfg.timeout_ms))
		.build()?;
	let url = format!("{}{}", cfg.base_url, cfg.path);
	let body = serde_json::json!({ "model": cfg.model, "query": query, "documents": docs });
	let res = client
		.post(url)
		.headers(auth_headers(&cfg.api_key, &cfg.default_headers)?)
		.json(&body)
		.send()
		.await?;
	let json: serde_json::Value = res.error_for_status()?.json().await?;
	parse_rerank_response(json, docs.len())
}

fn parse_rerank_response(json: serde_json::Value, doc_count: usize) -> Result<Vec<f32>> {
	let mut scores = vec![0.0f32; doc_count];
	let results = json
		.get("results")
		.or_else(|| json.get("data"))
		.and_then(|v| v.as_array())
		.ok_or_else(|| eyre!("Rerank response is missing results array."))?;

	for item in results {
		let index = item
			.get("index")
			.and_then(|v| v.as_u64())
			.ok_or_else(|| eyre!("Rerank result missing index."))? as usize;
		let score = item
			.get("relevance_score")
			.or_else(|| item.get("score"))
			.and_then(|v| v.as_f64())
			.ok_or_else(|| eyre!("Rerank result missing score."))? as f32;
		if index < scores.len() {
			scores[index] = score;
		}
	}

	Ok(scores)
}

#[cfg(test)]
mod tests {
	use super::parse_rerank_response;

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
}
