use std::{string::ToString, sync::Arc};

use serde_json::Value;

use crate::acceptance::{self, SpyExtractor, StubEmbedding, StubRerank};
use elf_service::{DocsPutRequest, DocsPutResponse, ElfService, Providers};
use elf_testkit::TestDatabase;

pub(crate) const TEST_CONTENT: &str =
	"ELF docs extension v1 stores evidence. Keyword: peregrine.\nSecond sentence for chunking.";

pub(crate) struct DocsContext {
	pub(crate) test_db: TestDatabase,
	pub(crate) service: ElfService,
}

pub(crate) async fn setup_docs_context() -> Option<DocsContext> {
	let Some(test_db) = acceptance::test_db().await else {
		eprintln!("Skipping docs_extension_v1; set ELF_PG_DSN to run this test.");

		return None;
	};
	let Some(qdrant_url) = acceptance::test_qdrant_url() else {
		eprintln!(
			"Skipping docs_extension_v1; set ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."
		);

		return None;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let cfg = acceptance::test_config(
		test_db.dsn().to_string(),
		qdrant_url,
		4_096,
		collection,
		docs_collection,
	);
	let providers = Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(StubRerank),
		Arc::new(SpyExtractor {
			calls: Arc::new(Default::default()),
			payload: serde_json::json!({ "notes": [] }),
		}),
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
	.expect("Failed to reset Qdrant memory collection.");
	acceptance::reset_qdrant_collection(
		&service.qdrant.client,
		&service.cfg.storage.qdrant.docs_collection,
		service.qdrant.vector_dim,
	)
	.await
	.expect("Failed to reset Qdrant docs collection.");

	Some(DocsContext { test_db, service })
}

pub(crate) async fn put_test_doc(service: &ElfService) -> DocsPutResponse {
	put_test_doc_with(
		service,
		"owner",
		"project_shared",
		None,
		"Docs v1",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
			"uri": "acceptance://knowledge/v1"
		}),
		TEST_CONTENT,
	)
	.await
}

pub(crate) async fn put_test_doc_with(
	service: &ElfService,
	agent_id: &str,
	scope: &str,
	doc_type: Option<&str>,
	title: &str,
	source_ref: Value,
	content: &str,
) -> DocsPutResponse {
	service
		.docs_put(DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: agent_id.to_string(),
			scope: scope.to_string(),
			doc_type: doc_type.map(ToString::to_string),
			title: Some(title.to_string()),
			write_policy: None,
			source_ref,
			content: content.to_string(),
		})
		.await
		.expect("Failed to put doc.")
}
