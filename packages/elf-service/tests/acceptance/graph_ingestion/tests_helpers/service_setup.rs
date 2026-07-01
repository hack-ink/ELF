use std::sync::{Arc, atomic::AtomicUsize};

use serde_json::Value;

use crate::acceptance::{
	self, SpyExtractor, StubEmbedding, StubRerank,
	graph_ingestion::tests_helpers::embedding::HashEmbedding,
};
use elf_service::{ElfService, Providers};
use elf_testkit::TestDatabase;

pub(in crate::acceptance::graph_ingestion) async fn build_test_db(
	test_name: &str,
) -> Option<TestDatabase> {
	let Some(test_db) = acceptance::test_db().await else {
		eprintln!("Skipping {test_name}; set ELF_PG_DSN to run.");

		return None;
	};
	let Some(_) = acceptance::test_qdrant_url() else {
		eprintln!("Skipping {test_name}; set ELF_QDRANT_URL to run.");

		return None;
	};

	Some(test_db)
}

pub(in crate::acceptance::graph_ingestion) async fn build_hash_service(
	test_db: &TestDatabase,
) -> ElfService {
	let qdrant_url = acceptance::test_qdrant_url().expect("Expected Qdrant test URL.");
	let providers = Providers::new(
		Arc::new(HashEmbedding { vector_dim: 4_096 }),
		Arc::new(StubRerank),
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

	acceptance::build_service(cfg, providers).await.expect("Failed to build service.")
}

pub(in crate::acceptance::graph_ingestion) async fn build_stub_service(
	test_db: &TestDatabase,
) -> ElfService {
	build_service_with_extractor_payload(test_db, serde_json::json!({ "notes": [] })).await
}

pub(in crate::acceptance::graph_ingestion) async fn build_service_with_extractor_payload(
	test_db: &TestDatabase,
	extractor_payload: Value,
) -> ElfService {
	let qdrant_url = acceptance::test_qdrant_url().expect("Expected Qdrant test URL.");
	let providers = Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(StubRerank),
		Arc::new(SpyExtractor { calls: Arc::new(AtomicUsize::new(0)), payload: extractor_payload }),
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

	acceptance::build_service(cfg, providers).await.expect("Failed to build service.")
}

pub(in crate::acceptance::graph_ingestion) async fn reset_service_db(service: &ElfService) {
	acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");
}
