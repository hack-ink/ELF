use color_eyre::{Result, eyre::eyre};

use crate::auth_headers;

pub async fn extract(
	cfg: &elf_config::LlmProviderConfig,
	messages: &[serde_json::Value],
) -> Result<serde_json::Value> {
	let client = reqwest::Client::builder()
		.timeout(std::time::Duration::from_millis(cfg.timeout_ms))
		.build()?;
	let url = format!("{}{}", cfg.base_url, cfg.path);

	for _ in 0..3 {
		let body = serde_json::json!({
			"model": cfg.model,
			"temperature": cfg.temperature,
			"messages": messages,
		});
		let res = client
			.post(&url)
			.headers(auth_headers(&cfg.api_key, &cfg.default_headers)?)
			.json(&body)
			.send()
			.await?;
		let json: serde_json::Value = res.error_for_status()?.json().await?;
		if let Ok(parsed) = parse_extractor_json(json) {
			return Ok(parsed);
		}
	}

	Err(eyre!("Extractor response is not valid JSON."))
}

fn parse_extractor_json(json: serde_json::Value) -> Result<serde_json::Value> {
	if let Some(content) = json
		.get("choices")
		.and_then(|v| v.as_array())
		.and_then(|arr| arr.first())
		.and_then(|choice| choice.get("message"))
		.and_then(|msg| msg.get("content"))
		.and_then(|c| c.as_str())
	{
		let parsed: serde_json::Value = serde_json::from_str(content)
			.map_err(|_| eyre!("Extractor content is not valid JSON."))?;
		return Ok(parsed);
	}

	if json.is_object() {
		return Ok(json);
	}

	Err(eyre!("Extractor response is missing JSON content."))
}

#[cfg(test)]
mod tests {
	use super::parse_extractor_json;

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
