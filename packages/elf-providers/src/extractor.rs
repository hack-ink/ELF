use std::time::Duration;

use reqwest::Client;
use serde_json::Value;

use crate::{Error, Result};

pub async fn extract(cfg: &elf_config::LlmProviderConfig, messages: &[Value]) -> Result<Value> {
	let client = Client::builder().timeout(Duration::from_millis(cfg.timeout_ms)).build()?;
	let url = format!("{}{}", cfg.api_base, cfg.path);

	for _ in 0..3 {
		let body = serde_json::json!({
			"model": cfg.model,
			"temperature": cfg.temperature,
			"messages": messages,
		});
		let res = client
			.post(&url)
			.headers(crate::auth_headers(&cfg.api_key, &cfg.default_headers)?)
			.json(&body)
			.send()
			.await?;
		let json: Value = res.error_for_status()?.json().await?;
		if let Ok(parsed) = parse_extractor_json(json) {
			return Ok(parsed);
		}
	}

	Err(Error::InvalidResponse { message: "Extractor response is not valid JSON.".to_string() })
}

fn parse_extractor_json(json: Value) -> Result<Value> {
	if let Some(content) = json
		.get("choices")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|choice| choice.get("message"))
		.and_then(|msg| msg.get("content"))
		.and_then(|c| c.as_str())
	{
		let parsed: Value = serde_json::from_str(content).map_err(|_| Error::InvalidResponse {
			message: "Extractor content is not valid JSON.".to_string(),
		})?;

		return Ok(parsed);
	}

	if json.is_object() {
		return Ok(json);
	}

	Err(Error::InvalidResponse {
		message: "Extractor response is missing JSON content.".to_string(),
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_choice_content_json() {
		let json = serde_json::json!({
			"choices": [
				{ "message": { "content": "{\"notes\": []}" } }
			]
		});
		let parsed = parse_extractor_json(json).expect("parse failed");
		assert!(parsed.get("notes").is_some());
	}
}
