use serde_json::Map;

use elf_config::{EmbeddingProviderConfig, LlmProviderConfig, ProviderConfig, Providers};

pub(crate) fn memory_policy_providers_config() -> Providers {
	Providers {
		embedding: embedding_provider_config(),
		rerank: rerank_provider_config(),
		llm_extractor: llm_extractor_provider_config(),
	}
}

fn embedding_provider_config() -> EmbeddingProviderConfig {
	EmbeddingProviderConfig {
		provider_id: "p".to_string(),
		api_base: "http://localhost".to_string(),
		api_key: "key".to_string(),
		path: "/".to_string(),
		model: "m".to_string(),
		dimensions: 3,
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}

fn rerank_provider_config() -> ProviderConfig {
	ProviderConfig {
		provider_id: "p".to_string(),
		api_base: "http://localhost".to_string(),
		api_key: "key".to_string(),
		path: "/".to_string(),
		model: "m".to_string(),
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}

fn llm_extractor_provider_config() -> LlmProviderConfig {
	LlmProviderConfig {
		provider_id: "p".to_string(),
		api_base: "http://localhost".to_string(),
		api_key: "key".to_string(),
		path: "/".to_string(),
		model: "m".to_string(),
		temperature: 0.1,
		timeout_ms: 1_000,
		default_headers: Map::new(),
	}
}
