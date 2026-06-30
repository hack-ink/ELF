use serde_json::Value;

use crate::{Error, Result};

pub(super) fn parse_rerank_response(json: Value, doc_count: usize) -> Result<Vec<f32>> {
	let results =
		json.get("results").or_else(|| json.get("data")).and_then(|v| v.as_array()).ok_or_else(
			|| Error::InvalidResponse {
				message: "Rerank response is missing results array.".to_string(),
			},
		)?;
	let mut scores = vec![0.0_f32; doc_count];

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
