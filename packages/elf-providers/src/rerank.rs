//! Rerank-provider client helpers.

mod local;
mod noise;
mod response;

use std::time::Duration;

use reqwest::Client;
use serde_json::Value;

use crate::Result;
use elf_config::ProviderConfig;

/// Reranks documents with the configured provider or local fallback implementation.
pub async fn rerank(cfg: &ProviderConfig, query: &str, docs: &[String]) -> Result<Vec<f32>> {
	if cfg.provider_id == "local" {
		return Ok(local::local_rerank_dispatch(cfg.model.as_str(), query, docs));
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

	response::parse_rerank_response(json, docs.len())
}

#[cfg(test)] mod tests;
