use std::sync::Arc;

use color_eyre::Result;

use crate::{
	BaselineRuntime, DeterministicEmbedding, ExternalEmbedding, NoopExtractor,
	RetrievalOrderRerank, TokenOverlapRerank,
};
use elf_config::Config;
use elf_service::Providers;

pub(crate) fn runtime_config(runtime: &BaselineRuntime) -> Result<Config> {
	let mut cfg = elf_config::load(&runtime.config_path)?;

	cfg.storage.postgres.dsn = runtime.dsn.clone();
	cfg.storage.postgres.pool_max_conns = 12;
	cfg.storage.qdrant.url = runtime.qdrant_url.clone();
	cfg.storage.qdrant.collection = runtime.collection.clone();
	cfg.storage.qdrant.docs_collection = runtime.docs_collection.clone();
	if std::env::var("ELF_REAL_WORLD_EXTERNAL_EMBEDDING").as_deref() == Ok("1") {
		let dimensions = std::env::var("EMBEDDING_DIMENSIONS")?.parse::<u32>()?;

		cfg.storage.qdrant.vector_dim = dimensions;
		cfg.providers.embedding.provider_id = "openai-compatible".to_string();
		cfg.providers.embedding.api_base = std::env::var("EMBEDDING_API_BASE")?;
		cfg.providers.embedding.api_key = std::env::var("EMBEDDING_API_KEY")?;
		cfg.providers.embedding.path = "/embeddings".to_string();
		cfg.providers.embedding.model = std::env::var("EMBEDDING_MODEL")?;
		cfg.providers.embedding.dimensions = dimensions;
		cfg.providers.embedding.timeout_ms = 180_000;
	} else {
		cfg.providers.embedding.provider_id = "local".to_string();
		cfg.providers.embedding.model = "local-hash".to_string();
		cfg.providers.embedding.dimensions = cfg.storage.qdrant.vector_dim;
	}
	cfg.providers.rerank.provider_id = "local".to_string();
	cfg.providers.rerank.model = "local-token-overlap".to_string();
	cfg.providers.llm_extractor.provider_id = "disabled".to_string();
	cfg.providers.llm_extractor.model = "disabled".to_string();
	cfg.context = None;

	Ok(cfg)
}

pub(crate) fn real_world_providers(cfg: &Config) -> Providers {
	if cfg.providers.embedding.provider_id == "openai-compatible" {
		return Providers::new(
			Arc::new(ExternalEmbedding),
			Arc::new(RetrievalOrderRerank),
			Arc::new(NoopExtractor),
		);
	}

	deterministic_providers(cfg.storage.qdrant.vector_dim)
}

pub(crate) fn deterministic_providers(vector_dim: u32) -> Providers {
	Providers::new(
		Arc::new(DeterministicEmbedding { vector_dim }),
		Arc::new(TokenOverlapRerank),
		Arc::new(NoopExtractor),
	)
}
