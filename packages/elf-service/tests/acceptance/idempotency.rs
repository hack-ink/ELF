use std::sync::{Arc, atomic::AtomicUsize};

use elf_service::{AddNoteInput, AddNoteRequest, NoteOp, Providers};

use super::{SpyExtractor, StubEmbedding, StubRerank};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn add_note_is_idempotent() {
	let Some(test_db) = super::test_db().await else {
		eprintln!("Skipping add_note_is_idempotent; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = super::test_qdrant_url() else {
		eprintln!("Skipping add_note_is_idempotent; set ELF_QDRANT_URL to run this test.");

		return;
	};
	let extractor = SpyExtractor {
		calls: Arc::new(AtomicUsize::new(0)),
		payload: serde_json::json!({ "notes": [] }),
	};
	let providers = Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(StubRerank),
		Arc::new(extractor),
	);
	let collection = test_db.collection_name("elf_acceptance");
	let cfg = super::test_config(test_db.dsn().to_string(), qdrant_url, 4_096, collection);
	let service = super::build_service(cfg, providers).await.expect("Failed to build service.");

	super::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	let request = AddNoteRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![AddNoteInput {
			note_type: "preference".to_string(),
			key: Some("preferred_language".to_string()),
			text: "Preference: Use English.".to_string(),
			structured: None,
			importance: 0.5,
			confidence: 0.9,
			ttl_days: None,
			source_ref: serde_json::json!({}),
		}],
	};
	let first = service.add_note(request.clone()).await.expect("First add_note failed.");
	let second = service.add_note(request).await.expect("Second add_note failed.");

	assert_eq!(first.results.len(), 1);
	assert_eq!(second.results.len(), 1);
	assert_eq!(second.results[0].op, NoteOp::None);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
