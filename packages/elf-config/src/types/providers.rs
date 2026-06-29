use serde::Deserialize;
use serde_json::{Map, Value};

/// Provider configuration bundle for all external model calls.
#[derive(Debug, Deserialize)]
pub struct Providers {
	/// Embedding provider used for vector generation.
	pub embedding: EmbeddingProviderConfig,
	/// Rerank provider used for late-stage scoring.
	pub rerank: ProviderConfig,
	/// LLM provider used by extraction flows such as `add_event`.
	pub llm_extractor: LlmProviderConfig,
}

/// Embedding-provider settings.
#[derive(Debug, Deserialize)]
pub struct EmbeddingProviderConfig {
	/// Provider implementation identifier.
	pub provider_id: String,
	/// Base URL for embedding API requests.
	pub api_base: String,
	/// Non-empty API key for embedding requests.
	pub api_key: String,
	/// Request path appended to `api_base`.
	pub path: String,
	/// Embedding model identifier.
	pub model: String,
	/// Expected embedding vector dimension.
	pub dimensions: u32,
	/// Request timeout in milliseconds.
	pub timeout_ms: u64,
	/// Extra HTTP headers sent with embedding requests.
	pub default_headers: Map<String, Value>,
}

/// Generic provider settings shared by non-embedding APIs such as rerank.
#[derive(Debug, Deserialize)]
pub struct ProviderConfig {
	/// Provider implementation identifier.
	pub provider_id: String,
	/// Base URL for provider API requests.
	pub api_base: String,
	/// Non-empty API key for provider requests.
	pub api_key: String,
	/// Request path appended to `api_base`.
	pub path: String,
	/// Provider model identifier.
	pub model: String,
	/// Request timeout in milliseconds.
	pub timeout_ms: u64,
	/// Extra HTTP headers sent with provider requests.
	pub default_headers: Map<String, Value>,
}

/// LLM extractor provider settings.
#[derive(Debug, Deserialize)]
pub struct LlmProviderConfig {
	/// Provider implementation identifier.
	pub provider_id: String,
	/// Base URL for extraction API requests.
	pub api_base: String,
	/// Non-empty API key for extraction requests.
	pub api_key: String,
	/// Request path appended to `api_base`.
	pub path: String,
	/// LLM model identifier.
	pub model: String,
	/// Sampling temperature for extraction requests.
	pub temperature: f32,
	/// Request timeout in milliseconds.
	pub timeout_ms: u64,
	/// Extra HTTP headers sent with extraction requests.
	pub default_headers: Map<String, Value>,
}
