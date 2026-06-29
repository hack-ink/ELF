use super::*;

#[derive(Debug)]
pub(super) struct DeterministicEmbedding {
	vector_dim: u32,
}
impl EmbeddingProvider for DeterministicEmbedding {
	fn embed<'a>(
		&'a self,
		_cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, elf_service::Result<Vec<Vec<f32>>>> {
		let dim = self.vector_dim;
		let vectors = texts.iter().map(|text| embed_text(text, dim)).collect();

		Box::pin(async move { Ok(vectors) })
	}
}

#[derive(Debug)]
pub(super) struct TokenOverlapRerank;
impl RerankProvider for TokenOverlapRerank {
	fn rerank<'a>(
		&'a self,
		_cfg: &'a ProviderConfig,
		query: &'a str,
		docs: &'a [String],
	) -> BoxFuture<'a, elf_service::Result<Vec<f32>>> {
		let query_terms = terms(query);
		let scores = docs
			.iter()
			.map(|doc| {
				let doc_terms = terms(doc);
				let hits = query_terms.intersection(&doc_terms).count() as f32;

				hits / query_terms.len().max(1) as f32
			})
			.collect();

		Box::pin(async move { Ok(scores) })
	}
}

#[derive(Debug)]
pub(super) struct NoopExtractor;
impl ExtractorProvider for NoopExtractor {
	fn extract<'a>(
		&'a self,
		_cfg: &'a LlmProviderConfig,
		_messages: &'a [Value],
	) -> BoxFuture<'a, elf_service::Result<Value>> {
		Box::pin(async move { Ok(serde_json::json!({ "notes": [] })) })
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum EmbeddingMode {
	Local,
	Provider,
}

pub(super) fn runtime_config(runtime: &BaselineRuntime) -> color_eyre::Result<Config> {
	let embedding_mode = embedding_mode()?;
	let mut cfg = elf_config::load(&runtime.config_path)?;

	cfg.storage.postgres.dsn = runtime.dsn.clone();
	cfg.storage.postgres.pool_max_conns = 12;
	cfg.storage.qdrant.url = runtime.qdrant_url.clone();
	cfg.storage.qdrant.collection = runtime.collection.clone();
	cfg.storage.qdrant.docs_collection = runtime.docs_collection.clone();

	if embedding_mode == EmbeddingMode::Provider {
		apply_provider_embedding_overrides(&mut cfg)?;

		cfg.storage.qdrant.vector_dim = cfg.providers.embedding.dimensions;
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

pub(super) fn deterministic_providers(vector_dim: u32) -> Providers {
	Providers::new(
		Arc::new(DeterministicEmbedding { vector_dim }),
		Arc::new(TokenOverlapRerank),
		Arc::new(NoopExtractor),
	)
}

pub(super) fn embedding_mode() -> color_eyre::Result<EmbeddingMode> {
	let raw = env::var("ELF_BASELINE_ELF_EMBEDDING_MODE")
		.unwrap_or_else(|_| "local".to_string())
		.to_ascii_lowercase();

	match raw.as_str() {
		"local" | "deterministic" => Ok(EmbeddingMode::Local),
		"provider" | "production" => Ok(EmbeddingMode::Provider),
		_ => Err(eyre::eyre!(
			"Unsupported ELF_BASELINE_ELF_EMBEDDING_MODE={raw:?}; use local or provider."
		)),
	}
}

pub(super) fn apply_provider_embedding_overrides(cfg: &mut Config) -> color_eyre::Result<()> {
	apply_env_string(
		&mut cfg.providers.embedding.provider_id,
		&[
			"ELF_BASELINE_ELF_EMBEDDING_PROVIDER_ID",
			"QWEN_EMBEDDING_PROVIDER_ID",
			"EMBEDDING_PROVIDER_ID",
		],
	);
	apply_env_string(
		&mut cfg.providers.embedding.api_base,
		&[
			"ELF_BASELINE_ELF_EMBEDDING_API_BASE",
			"QWEN_EMBEDDING_API_BASE",
			"DASHSCOPE_API_BASE",
			"EMBEDDING_API_BASE",
		],
	);
	apply_env_string(
		&mut cfg.providers.embedding.api_key,
		&[
			"ELF_BASELINE_ELF_EMBEDDING_API_KEY",
			"QWEN_API_KEY",
			"DASHSCOPE_API_KEY",
			"EMBEDDING_API_KEY",
		],
	);
	apply_env_string(
		&mut cfg.providers.embedding.path,
		&["ELF_BASELINE_ELF_EMBEDDING_PATH", "QWEN_EMBEDDING_PATH", "EMBEDDING_PATH"],
	);
	apply_env_string(
		&mut cfg.providers.embedding.model,
		&["ELF_BASELINE_ELF_EMBEDDING_MODEL", "QWEN_EMBEDDING_MODEL", "EMBEDDING_MODEL"],
	);

	if let Some(dimensions) = env_u32(&[
		"ELF_BASELINE_ELF_EMBEDDING_DIMENSIONS",
		"QWEN_EMBEDDING_DIMENSIONS",
		"DASHSCOPE_EMBEDDING_DIMENSIONS",
		"EMBEDDING_DIMENSIONS",
	]) {
		cfg.providers.embedding.dimensions = dimensions;
	}
	if let Some(timeout_ms) = env_u64(&[
		"ELF_BASELINE_ELF_EMBEDDING_TIMEOUT_MS",
		"QWEN_EMBEDDING_TIMEOUT_MS",
		"EMBEDDING_TIMEOUT_MS",
	]) {
		cfg.providers.embedding.timeout_ms = timeout_ms;
	} else {
		cfg.providers.embedding.timeout_ms = cfg.providers.embedding.timeout_ms.max(30_000);
	}

	if cfg.providers.embedding.provider_id == "local" {
		if env_string(&["ELF_BASELINE_ELF_EMBEDDING_API_KEY", "QWEN_API_KEY"]).is_some() {
			cfg.providers.embedding.provider_id = "qwen".to_string();
		} else if env_string(&["DASHSCOPE_API_KEY"]).is_some() {
			cfg.providers.embedding.provider_id = "dashscope".to_string();
		} else if env_string(&["EMBEDDING_API_KEY"]).is_some() {
			cfg.providers.embedding.provider_id = "provider".to_string();
		}
	}
	if cfg.providers.embedding.provider_id == "local" {
		return Err(eyre::eyre!(
			"Provider embedding mode requires a non-local provider id or QWEN_API_KEY/DASHSCOPE_API_KEY/EMBEDDING_API_KEY."
		));
	}
	if cfg.providers.embedding.api_base.trim().is_empty()
		|| cfg.providers.embedding.api_base == "http://127.0.0.1"
	{
		return Err(eyre::eyre!(
			"Provider embedding mode requires ELF_BASELINE_ELF_EMBEDDING_API_BASE, QWEN_EMBEDDING_API_BASE, DASHSCOPE_API_BASE, or EMBEDDING_API_BASE."
		));
	}
	if cfg.providers.embedding.api_key.trim().is_empty()
		|| cfg.providers.embedding.api_key == "local-dev-placeholder"
	{
		return Err(eyre::eyre!(
			"Provider embedding mode requires ELF_BASELINE_ELF_EMBEDDING_API_KEY, QWEN_API_KEY, DASHSCOPE_API_KEY, or EMBEDDING_API_KEY."
		));
	}
	if cfg.providers.embedding.model == "local-hash"
		|| cfg.providers.embedding.model.trim().is_empty()
	{
		return Err(eyre::eyre!(
			"Provider embedding mode requires ELF_BASELINE_ELF_EMBEDDING_MODEL, QWEN_EMBEDDING_MODEL, or EMBEDDING_MODEL."
		));
	}
	if cfg.providers.embedding.dimensions == 0 {
		return Err(eyre::eyre!(
			"Provider embedding dimensions must be greater than zero; set ELF_BASELINE_ELF_EMBEDDING_DIMENSIONS, QWEN_EMBEDDING_DIMENSIONS, DASHSCOPE_EMBEDDING_DIMENSIONS, or EMBEDDING_DIMENSIONS."
		));
	}

	Ok(())
}

pub(super) fn embedding_runtime_report(cfg: &Config) -> EmbeddingRuntimeReport {
	EmbeddingRuntimeReport {
		mode: embedding_mode().unwrap_or(EmbeddingMode::Local),
		provider_id: cfg.providers.embedding.provider_id.clone(),
		model: cfg.providers.embedding.model.clone(),
		dimensions: cfg.providers.embedding.dimensions,
		timeout_ms: cfg.providers.embedding.timeout_ms,
		api_base: cfg.providers.embedding.api_base.clone(),
		path: cfg.providers.embedding.path.clone(),
	}
}

pub(super) fn apply_env_string(target: &mut String, names: &[&str]) {
	if let Some(value) = env_string(names) {
		*target = value;
	}
}

pub(super) fn env_string(names: &[&str]) -> Option<String> {
	names.iter().find_map(|name| {
		env::var(name).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
	})
}

pub(super) fn env_u32(names: &[&str]) -> Option<u32> {
	env_string(names).and_then(|value| value.parse::<u32>().ok())
}

pub(super) fn env_u64(names: &[&str]) -> Option<u64> {
	env_string(names).and_then(|value| value.parse::<u64>().ok())
}
