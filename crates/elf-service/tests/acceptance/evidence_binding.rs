use std::sync::Arc;

use super::{SpyExtractor, StubEmbedding, StubRerank, build_service, test_config, test_dsn, test_qdrant_url};

#[tokio::test]
async fn rejects_invalid_evidence_quote() {
	let _guard = super::test_lock().await;
    let Some(dsn) = test_dsn() else {
        eprintln!("Skipping rejects_invalid_evidence_quote; set ELF_PG_DSN to run this test.");
        return;
    };
    let Some(qdrant_url) = test_qdrant_url() else {
        eprintln!("Skipping rejects_invalid_evidence_quote; set ELF_QDRANT_URL to run this test.");
        return;
    };

    let extractor_payload = serde_json::json!({
        "notes": [
            {
                "type": "fact",
                "key": "project_workflow",
                "text": "Fact: The workflow uses TODO markers.",
                "importance": 0.5,
                "confidence": 0.8,
                "ttl_days": null,
                "scope_suggestion": "agent_private",
                "evidence": [
                    { "message_index": 0, "quote": "This quote does not exist." }
                ],
                "reason": "test"
            }
        ]
    });

    let extractor = SpyExtractor {
        calls: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        payload: extractor_payload,
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

    let request = elf_service::AddEventRequest {
        tenant_id: "t".to_string(),
        project_id: "p".to_string(),
        agent_id: "a".to_string(),
        scope: Some("agent_private".to_string()),
        dry_run: Some(false),
        messages: vec![elf_service::EventMessage {
            role: "user".to_string(),
            content: "This is a message without the expected quote.".to_string(),
            ts: None,
            msg_id: None,
        }],
    };

    let response = service
        .add_event(request)
        .await
        .expect("add_event failed.");
    assert_eq!(response.results.len(), 1);
    let result = &response.results[0];
    assert_eq!(result.op, elf_service::NoteOp::Rejected);
    assert_eq!(
        result.reason_code.as_deref(),
        Some(elf_service::REJECT_EVIDENCE_MISMATCH)
    );
}
