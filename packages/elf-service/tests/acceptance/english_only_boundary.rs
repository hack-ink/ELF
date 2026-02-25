use std::sync::{Arc, atomic::AtomicUsize};

use crate::acceptance::{SpyExtractor, StubEmbedding, StubRerank};
use elf_service::{
	AddEventRequest, AddNoteInput, AddNoteRequest, ElfService, Error, EventMessage, Providers,
	SearchRequest,
};

async fn build_test_service(
	dsn: String,
	qdrant_url: String,
	collection: String,
	docs_collection: String,
) -> Option<ElfService> {
	let extractor = SpyExtractor {
		calls: Arc::new(AtomicUsize::new(0)),
		payload: serde_json::json!({ "notes": [] }),
	};
	let providers = Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(StubRerank),
		Arc::new(extractor),
	);
	let cfg = crate::acceptance::test_config(dsn, qdrant_url, 4_096, collection, docs_collection);
	let service =
		crate::acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	crate::acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	Some(service)
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_non_english_in_add_note() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping english_only_boundary; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!("Skipping english_only_boundary; set ELF_QDRANT_URL to run this test.");

		return;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let Some(service) =
		build_test_service(test_db.dsn().to_string(), qdrant_url, collection, docs_collection)
			.await
	else {
		return;
	};
	let request = AddNoteRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![AddNoteInput {
			r#type: "fact".to_string(),
			key: None,
			text: "你好".to_string(),
			structured: None,
			importance: 0.4,
			confidence: 0.9,
			ttl_days: None,
			source_ref: serde_json::json!({}),
		}],
	};
	let result = service.add_note(request).await;

	match result {
		Err(Error::NonEnglishInput { field }) => {
			assert_eq!(field, "$.notes[0].text");
		},
		other => panic!("Expected NonEnglishInput, got {other:?}"),
	}

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_cyrillic_in_add_note() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping english_only_boundary; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!("Skipping english_only_boundary; set ELF_QDRANT_URL to run this test.");

		return;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let Some(service) =
		build_test_service(test_db.dsn().to_string(), qdrant_url, collection, docs_collection)
			.await
	else {
		return;
	};
	let request = AddNoteRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![AddNoteInput {
			r#type: "fact".to_string(),
			key: None,
			text: "Привет мир".to_string(),
			structured: None,
			importance: 0.4,
			confidence: 0.9,
			ttl_days: None,
			source_ref: serde_json::json!({}),
		}],
	};
	let result = service.add_note(request).await;

	match result {
		Err(Error::NonEnglishInput { field }) => {
			assert_eq!(field, "$.notes[0].text");
		},
		other => panic!("Expected NonEnglishInput, got {other:?}"),
	}

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_non_english_in_add_event() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping english_only_boundary; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!("Skipping english_only_boundary; set ELF_QDRANT_URL to run this test.");

		return;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let Some(service) =
		build_test_service(test_db.dsn().to_string(), qdrant_url, collection, docs_collection)
			.await
	else {
		return;
	};
	let request = AddEventRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: Some("agent_private".to_string()),
		dry_run: Some(true),
		messages: vec![EventMessage {
			role: "user".to_string(),
			content: "こんにちは".to_string(),
			ts: None,
			msg_id: None,
		}],
	};
	let result = service.add_event(request).await;

	match result {
		Err(Error::NonEnglishInput { field }) => {
			assert_eq!(field, "$.messages[0].content");
		},
		other => panic!("Expected NonEnglishInput, got {other:?}"),
	}

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_cyrillic_in_add_event() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping english_only_boundary; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!("Skipping english_only_boundary; set ELF_QDRANT_URL to run this test.");

		return;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let Some(service) =
		build_test_service(test_db.dsn().to_string(), qdrant_url, collection, docs_collection)
			.await
	else {
		return;
	};
	let request = AddEventRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: Some("agent_private".to_string()),
		dry_run: Some(true),
		messages: vec![EventMessage {
			role: "user".to_string(),
			content: "Это не английский текст.".to_string(),
			ts: None,
			msg_id: None,
		}],
	};
	let result = service.add_event(request).await;

	match result {
		Err(Error::NonEnglishInput { field }) => {
			assert_eq!(field, "$.messages[0].content");
		},
		other => panic!("Expected NonEnglishInput, got {other:?}"),
	}

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_non_english_in_search() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping english_only_boundary; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!("Skipping english_only_boundary; set ELF_QDRANT_URL to run this test.");

		return;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let Some(service) =
		build_test_service(test_db.dsn().to_string(), qdrant_url, collection, docs_collection)
			.await
	else {
		return;
	};
	let request = SearchRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		token_id: None,
		read_profile: "private_only".to_string(),
		payload_level: Default::default(),
		query: "안녕하세요".to_string(),
		top_k: Some(5),
		candidate_k: Some(10),
		record_hits: Some(false),
		ranking: None,
	};
	let result = service.search(request).await;

	match result {
		Err(Error::NonEnglishInput { field }) => {
			assert_eq!(field, "$.query");
		},
		other => panic!("Expected NonEnglishInput, got {other:?}"),
	}

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_cyrillic_in_search() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping english_only_boundary; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!("Skipping english_only_boundary; set ELF_QDRANT_URL to run this test.");

		return;
	};
	let collection = test_db.collection_name("elf_acceptance");
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let Some(service) =
		build_test_service(test_db.dsn().to_string(), qdrant_url, collection, docs_collection)
			.await
	else {
		return;
	};
	let request = SearchRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		token_id: None,
		read_profile: "private_only".to_string(),
		payload_level: Default::default(),
		query: "Привет".to_string(),
		top_k: Some(5),
		candidate_k: Some(10),
		record_hits: Some(false),
		ranking: None,
	};
	let result = service.search(request).await;

	match result {
		Err(Error::NonEnglishInput { field }) => {
			assert_eq!(field, "$.query");
		},
		other => panic!("Expected NonEnglishInput, got {other:?}"),
	}

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
