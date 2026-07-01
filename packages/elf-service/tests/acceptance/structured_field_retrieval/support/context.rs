use std::sync::{Arc, atomic::AtomicUsize};

use crate::acceptance::{self, SpyExtractor, StubEmbedding};
use elf_config::ProviderConfig;
use elf_service::{BoxFuture, ElfService, Providers, RerankProvider, Result};
use elf_testkit::TestDatabase;

pub(crate) struct TestContext {
	pub(crate) service: ElfService,
	pub(crate) test_db: TestDatabase,
	pub(crate) embedding_version: String,
}

struct KeywordRerank {
	keyword: &'static str,
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

pub(crate) async fn setup_context(test_name: &str) -> Option<TestContext> {
	let Some(test_db) = acceptance::test_db().await else {
		eprintln!("Skipping {test_name}; set ELF_PG_DSN to run this test.");

		return None;
	};
	let Some(qdrant_url) = acceptance::test_qdrant_url() else {
		eprintln!("Skipping {test_name}; set ELF_QDRANT_URL to run this test.");

		return None;
	};
	let providers = Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(KeywordRerank { keyword: "ZEBRA" }),
		Arc::new(SpyExtractor {
			calls: Arc::new(AtomicUsize::new(0)),
			payload: serde_json::json!({ "notes": [] }),
		}),
	);
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let cfg = acceptance::test_config(
		test_db.dsn().to_string(),
		qdrant_url,
		4_096,
		collection,
		docs_collection,
	);
	let service =
		acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");
	acceptance::reset_qdrant_collection(
		&service.qdrant.client,
		&service.qdrant.collection,
		service.qdrant.vector_dim,
	)
	.await
	.expect("Failed to reset Qdrant collection.");

	let embedding_version = format!(
		"{}:{}:{}",
		service.cfg.providers.embedding.provider_id,
		service.cfg.providers.embedding.model,
		service.cfg.storage.qdrant.vector_dim
	);

	Some(TestContext { service, test_db, embedding_version })
}
