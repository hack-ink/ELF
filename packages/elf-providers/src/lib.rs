pub mod embedding;
pub mod extractor;
pub mod rerank;

mod error;

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName};
use serde_json::{Map, Value};

pub use error::{Error, Result};

pub fn auth_headers(api_key: &str, default_headers: &Map<String, Value>) -> Result<HeaderMap> {
	let mut headers = HeaderMap::new();
	headers.insert(AUTHORIZATION, format!("Bearer {api_key}").parse()?);
	for (key, value) in default_headers {
		let Some(raw) = value.as_str() else {
			return Err(Error::InvalidConfig {
				message: "Default header values must be strings.".to_string(),
			});
		};
		headers.insert(HeaderName::from_bytes(key.as_bytes())?, raw.parse()?);
	}
	Ok(headers)
}
