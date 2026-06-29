use std::{future::Future, pin::Pin, sync::Arc};

use serde_json::Value;

use crate::{Error, Result};
use elf_config::{EmbeddingProviderConfig, LlmProviderConfig, ProviderConfig};
use elf_providers::{embedding, extractor, rerank};

/// Boxed future type used by provider traits.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Embedding provider contract used by the service layer.
pub trait EmbeddingProvider
where
	Self: Send + Sync,
{
	/// Embeds one or more texts into dense vectors.
	fn embed<'a>(
		&'a self,
		cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, Result<Vec<Vec<f32>>>>;
}

/// Rerank provider contract used by the service layer.
pub trait RerankProvider
where
	Self: Send + Sync,
{
	/// Scores candidate documents for one query.
	fn rerank<'a>(
		&'a self,
		cfg: &'a ProviderConfig,
		query: &'a str,
		docs: &'a [String],
	) -> BoxFuture<'a, Result<Vec<f32>>>;
}

/// Extractor provider contract used by the service layer.
pub trait ExtractorProvider
where
	Self: Send + Sync,
{
	/// Extracts structured JSON output from a message transcript.
	fn extract<'a>(
		&'a self,
		cfg: &'a LlmProviderConfig,
		messages: &'a [Value],
	) -> BoxFuture<'a, Result<Value>>;
}

/// Provider bundle used by `ElfService`.
#[derive(Clone)]
pub struct Providers {
	/// Dense embedding provider implementation.
	pub embedding: Arc<dyn EmbeddingProvider>,
	/// Rerank provider implementation.
	pub rerank: Arc<dyn RerankProvider>,
	/// Structured extraction provider implementation.
	pub extractor: Arc<dyn ExtractorProvider>,
}
impl Providers {
	/// Builds a provider bundle from explicit provider implementations.
	pub fn new(
		embedding: Arc<dyn EmbeddingProvider>,
		rerank: Arc<dyn RerankProvider>,
		extractor: Arc<dyn ExtractorProvider>,
	) -> Self {
		Self { embedding, rerank, extractor }
	}
}

impl Default for Providers {
	fn default() -> Self {
		let provider = Arc::new(DefaultProviders);

		Self { embedding: provider.clone(), rerank: provider.clone(), extractor: provider }
	}
}

struct DefaultProviders;
impl EmbeddingProvider for DefaultProviders {
	fn embed<'a>(
		&'a self,
		cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, Result<Vec<Vec<f32>>>> {
		Box::pin(async move {
			embedding::embed(cfg, texts)
				.await
				.map_err(|err| Error::Provider { message: err.to_string() })
		})
	}
}

impl RerankProvider for DefaultProviders {
	fn rerank<'a>(
		&'a self,
		cfg: &'a ProviderConfig,
		query: &'a str,
		docs: &'a [String],
	) -> BoxFuture<'a, Result<Vec<f32>>> {
		Box::pin(async move {
			rerank::rerank(cfg, query, docs)
				.await
				.map_err(|err| Error::Provider { message: err.to_string() })
		})
	}
}

impl ExtractorProvider for DefaultProviders {
	fn extract<'a>(
		&'a self,
		cfg: &'a LlmProviderConfig,
		messages: &'a [Value],
	) -> BoxFuture<'a, Result<Value>> {
		Box::pin(async move {
			extractor::extract(cfg, messages)
				.await
				.map_err(|err| Error::Provider { message: err.to_string() })
		})
	}
}
