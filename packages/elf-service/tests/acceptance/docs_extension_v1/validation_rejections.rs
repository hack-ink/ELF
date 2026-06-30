use std::sync::Arc;

use crate::acceptance::{
	self, SpyExtractor, StubEmbedding, StubRerank, docs_extension_v1::TEST_CONTENT,
};
use elf_service::{DocsPutRequest, Error, Providers};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_put_rejects_non_english_source_ref() {
	let Some(test_db) = acceptance::test_db().await else {
		eprintln!("Skipping docs_extension_v1; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = acceptance::test_qdrant_url() else {
		eprintln!(
			"Skipping docs_extension_v1; set ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."
		);

		return;
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
	let result = service
		.docs_put(DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "owner".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: Some("Docs rejection sample".to_string()),
			write_policy: None,
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
				"notes": "你好"
			}),
			content: TEST_CONTENT.to_string(),
		})
		.await;

	match result {
		Err(Error::NonEnglishInput { field }) => {
			assert_eq!(field, "$.source_ref[\"notes\"]");
		},
		other => panic!("Expected NonEnglishInput, got {other:?}"),
	}

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_put_rejects_missing_and_invalid_source_ref() {
	let Some(test_db) = acceptance::test_db().await else {
		eprintln!("Skipping docs_extension_v1; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = acceptance::test_qdrant_url() else {
		eprintln!(
			"Skipping docs_extension_v1; set ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."
		);

		return;
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
	let result = service
		.docs_put(DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "owner".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: Some("Docs rejection sample".to_string()),
			write_policy: None,
			source_ref: serde_json::json!("legacy-shape"),
			content: TEST_CONTENT.to_string(),
		})
		.await;

	match result {
		Err(Error::InvalidRequest { message }) => {
			assert!(message.contains("source_ref must be a JSON object"));
		},
		other => panic!("Expected InvalidRequest for non-object source_ref, got {other:?}"),
	}

	let result = service
		.docs_put(DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "owner".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: Some("Docs rejection sample".to_string()),
			write_policy: None,
			source_ref: serde_json::json!({
				"schema": "source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
			}),
			content: TEST_CONTENT.to_string(),
		})
		.await;

	match result {
		Err(Error::InvalidRequest { message }) => {
			assert!(message.contains("doc_source_ref/v1"));
		},
		other => panic!("Expected InvalidRequest for wrong source_ref schema, got {other:?}"),
	}

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
