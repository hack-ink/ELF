use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

use serde_json::Value;

use elf_config::{EmbeddingProviderConfig, LlmProviderConfig, ProviderConfig};
use elf_service::{BoxFuture, EmbeddingProvider, ExtractorProvider, RerankProvider, Result};

pub(crate) struct DummyEmbedding;
impl EmbeddingProvider for DummyEmbedding {
	fn embed<'a>(
		&'a self,
		cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, Result<Vec<Vec<f32>>>> {
		let dim = (cfg.dimensions as usize).max(1);
		let vec = vec![0.0; dim];

		Box::pin(async move { Ok(vec![vec; texts.len()]) })
	}
}

pub(crate) struct DummyRerank;
impl RerankProvider for DummyRerank {
	fn rerank<'a>(
		&'a self,
		_cfg: &'a ProviderConfig,
		_query: &'a str,
		docs: &'a [String],
	) -> BoxFuture<'a, Result<Vec<f32>>> {
		let scores = vec![0.0; docs.len()];

		Box::pin(async move { Ok(scores) })
	}
}

pub(crate) struct SpyExtractor {
	calls: Arc<AtomicUsize>,
}
impl SpyExtractor {
	pub(crate) fn new() -> Self {
		Self { calls: Arc::new(AtomicUsize::new(0)) }
	}

	pub(crate) fn count(&self) -> usize {
		self.calls.load(Ordering::SeqCst)
	}
}
impl ExtractorProvider for SpyExtractor {
	fn extract<'a>(
		&'a self,
		_cfg: &'a LlmProviderConfig,
		_messages: &'a [Value],
	) -> BoxFuture<'a, Result<Value>> {
		self.calls.fetch_add(1, Ordering::SeqCst);

		Box::pin(async move { Ok(serde_json::json!({ "notes": [] })) })
	}
}
