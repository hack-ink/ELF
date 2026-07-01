use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

use serde_json::Value;

use elf_config::{EmbeddingProviderConfig, LlmProviderConfig, ProviderConfig};
use elf_service::{BoxFuture, EmbeddingProvider, ExtractorProvider, RerankProvider, Result};

pub struct StubEmbedding {
	pub vector_dim: u32,
}
impl EmbeddingProvider for StubEmbedding {
	fn embed<'a>(
		&'a self,
		_cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, Result<Vec<Vec<f32>>>> {
		let dim = self.vector_dim as usize;
		let vectors = texts.iter().map(|_| vec![0.0; dim]).collect();

		Box::pin(async move { Ok(vectors) })
	}
}

pub struct SpyEmbedding {
	pub vector_dim: u32,
	pub calls: Arc<AtomicUsize>,
}
impl EmbeddingProvider for SpyEmbedding {
	fn embed<'a>(
		&'a self,
		_cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, Result<Vec<Vec<f32>>>> {
		self.calls.fetch_add(1, Ordering::SeqCst);

		let dim = self.vector_dim as usize;
		let vectors = texts.iter().map(|_| vec![0.0; dim]).collect();

		Box::pin(async move { Ok(vectors) })
	}
}

pub struct StubRerank;
impl RerankProvider for StubRerank {
	fn rerank<'a>(
		&'a self,
		_cfg: &'a ProviderConfig,
		_query: &'a str,
		docs: &'a [String],
	) -> BoxFuture<'a, Result<Vec<f32>>> {
		let scores = vec![0.5; docs.len()];

		Box::pin(async move { Ok(scores) })
	}
}

pub struct SpyExtractor {
	pub calls: Arc<AtomicUsize>,
	pub payload: Value,
}
impl ExtractorProvider for SpyExtractor {
	fn extract<'a>(
		&'a self,
		_cfg: &'a LlmProviderConfig,
		_messages: &'a [Value],
	) -> BoxFuture<'a, Result<Value>> {
		let payload = self.payload.clone();

		self.calls.fetch_add(1, Ordering::SeqCst);

		Box::pin(async move { Ok(payload) })
	}
}
