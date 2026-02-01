pub mod embedding;
pub mod extractor;
pub mod rerank;

use color_eyre::{eyre::eyre, Result};

pub fn auth_headers(
    api_key: &str,
    default_headers: &serde_json::Map<String, serde_json::Value>,
) -> Result<reqwest::header::HeaderMap> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {api_key}").parse()?,
    );
    for (key, value) in default_headers {
        let Some(raw) = value.as_str() else {
            return Err(eyre!("Default header values must be strings."));
        };
        headers.insert(
            reqwest::header::HeaderName::from_bytes(key.as_bytes())?,
            raw.parse()?,
        );
    }
    Ok(headers)
}
