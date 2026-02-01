use std::sync::Arc;

use super::{SpyExtractor, StubEmbedding, StubRerank, build_service, test_config, test_dsn, test_qdrant_url};

#[tokio::test]
async fn rejects_cjk_in_add_note() {
    let Some(dsn) = test_dsn() else {
        eprintln!("Skipping rejects_cjk_in_add_note; set ELF_TEST_PG_DSN to run this test.");
        return;
    };
    let Some(qdrant_url) = test_qdrant_url() else {
        eprintln!("Skipping rejects_cjk_in_add_note; set ELF_TEST_QDRANT_URL to run this test.");
        return;
    };

    let extractor = SpyExtractor {
        calls: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
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

    let request = elf_service::AddNoteRequest {
        tenant_id: "t".to_string(),
        project_id: "p".to_string(),
        agent_id: "a".to_string(),
        scope: "agent_private".to_string(),
        notes: vec![elf_service::AddNoteInput {
            note_type: "fact".to_string(),
            key: None,
            text: "你好".to_string(),
            importance: 0.4,
            confidence: 0.9,
            ttl_days: None,
            source_ref: serde_json::json!({}),
        }],
    };

    let result = service.add_note(request).await;
    match result {
        Err(elf_service::ServiceError::NonEnglishInput { .. }) => {}
        other => panic!("Expected NonEnglishInput, got {other:?}"),
    }
}
