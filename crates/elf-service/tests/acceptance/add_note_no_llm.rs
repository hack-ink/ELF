use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use super::{SpyExtractor, StubEmbedding, StubRerank, build_service, test_config, test_dsn, test_qdrant_url};

#[tokio::test]
async fn add_note_does_not_call_llm() {
	let _guard = super::test_lock().await;
    let Some(dsn) = test_dsn() else {
        eprintln!("Skipping add_note_does_not_call_llm; set ELF_PG_DSN to run this test.");
        return;
    };
    let Some(qdrant_url) = test_qdrant_url() else {
        eprintln!("Skipping add_note_does_not_call_llm; set ELF_QDRANT_URL to run this test.");
        return;
    };

    let calls = Arc::new(AtomicUsize::new(0));
    let extractor = SpyExtractor {
        calls: calls.clone(),
        payload: serde_json::json!({ "notes": [] }),
    };
    let providers = elf_service::Providers::new(
        Arc::new(StubEmbedding { vector_dim: 3 }),
        Arc::new(StubRerank),
        Arc::new(extractor),
    );

    let cfg = test_config(dsn, qdrant_url, 3);
    let service = build_service(cfg, providers)
        .await
        .expect("Failed to build service.");
	super::reset_db(&service.db.pool)
		.await
		.expect("Failed to reset test database.");

    let request = elf_service::AddNoteRequest {
        tenant_id: "t".to_string(),
        project_id: "p".to_string(),
        agent_id: "a".to_string(),
        scope: "agent_private".to_string(),
        notes: vec![elf_service::AddNoteInput {
            note_type: "preference".to_string(),
            key: Some("preferred_language".to_string()),
            text: "Preference: Use English.".to_string(),
            importance: 0.5,
            confidence: 0.9,
            ttl_days: None,
            source_ref: serde_json::json!({}),
        }],
    };

    service.add_note(request).await.expect("add_note failed.");
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}
