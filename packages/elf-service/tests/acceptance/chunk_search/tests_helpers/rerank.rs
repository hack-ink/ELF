use std::sync::{Arc, atomic::AtomicUsize};

use crate::acceptance::{SpyExtractor, StubEmbedding};
use elf_config::ProviderConfig;
use elf_service::{BoxFuture, Providers, RerankProvider, Result};

pub(in super::super) struct KeywordRerank {
	pub(in super::super) keyword: &'static str,
}
impl RerankProvider for KeywordRerank {
	fn rerank<'a>(
		&'a self,
		_cfg: &'a ProviderConfig,
		_query: &'a str,
		docs: &'a [String],
	) -> BoxFuture<'a, Result<Vec<f32>>> {
		let keyword = self.keyword;

		Box::pin(async move {
			Ok(docs.iter().map(|doc| if doc.contains(keyword) { 1.0 } else { 0.1 }).collect())
		})
	}
}

pub(in super::super) fn build_providers<R>(rerank: R) -> Providers
where
	R: RerankProvider + Send + Sync + 'static,
{
	Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(rerank),
		Arc::new(SpyExtractor {
			calls: Arc::new(AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
	)
}
