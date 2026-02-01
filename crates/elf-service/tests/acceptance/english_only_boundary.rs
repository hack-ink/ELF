use std::sync::Arc;

use super::{SpyExtractor, StubEmbedding, StubRerank, build_service, test_config, test_dsn, test_qdrant_url};

#[tokio::test]
async fn rejects_cjk_in_add_note() {
	let Some(dsn) = test_dsn() else {
		eprintln!("Skipping english_only_boundary; set ELF_PG_DSN to run this test.");
		return;
	};
	let Some(qdrant_url) = test_qdrant_url() else {
		eprintln!("Skipping english_only_boundary; set ELF_QDRANT_URL to run this test.");
		return;
	};
	let _guard = super::test_lock(&dsn)
		.await
		.expect("Failed to acquire test lock.");
    let Some(service) = build_test_service(dsn, qdrant_url).await else {
        return;
    };

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
        Err(elf_service::ServiceError::NonEnglishInput { field }) => {
            assert_eq!(field, "$.notes[0].text");
        }
        other => panic!("Expected NonEnglishInput, got {other:?}"),
    }
}

#[tokio::test]
async fn rejects_cjk_in_add_event() {
	let Some(dsn) = test_dsn() else {
		eprintln!("Skipping english_only_boundary; set ELF_PG_DSN to run this test.");
		return;
	};
	let Some(qdrant_url) = test_qdrant_url() else {
		eprintln!("Skipping english_only_boundary; set ELF_QDRANT_URL to run this test.");
		return;
	};
	let _guard = super::test_lock(&dsn)
		.await
		.expect("Failed to acquire test lock.");
    let Some(service) = build_test_service(dsn, qdrant_url).await else {
        return;
    };

    let request = elf_service::AddEventRequest {
        tenant_id: "t".to_string(),
        project_id: "p".to_string(),
        agent_id: "a".to_string(),
        scope: Some("agent_private".to_string()),
        dry_run: Some(true),
        messages: vec![elf_service::EventMessage {
            role: "user".to_string(),
            content: "こんにちは".to_string(),
            ts: None,
            msg_id: None,
        }],
    };

    let result = service.add_event(request).await;
    match result {
        Err(elf_service::ServiceError::NonEnglishInput { field }) => {
            assert_eq!(field, "$.messages[0].content");
        }
        other => panic!("Expected NonEnglishInput, got {other:?}"),
    }
}

#[tokio::test]
async fn rejects_cjk_in_search() {
	let Some(dsn) = test_dsn() else {
		eprintln!("Skipping english_only_boundary; set ELF_PG_DSN to run this test.");
		return;
	};
	let Some(qdrant_url) = test_qdrant_url() else {
		eprintln!("Skipping english_only_boundary; set ELF_QDRANT_URL to run this test.");
		return;
	};
	let _guard = super::test_lock(&dsn)
		.await
		.expect("Failed to acquire test lock.");
    let Some(service) = build_test_service(dsn, qdrant_url).await else {
        return;
    };

    let request = elf_service::SearchRequest {
        tenant_id: "t".to_string(),
        project_id: "p".to_string(),
        agent_id: "a".to_string(),
        read_profile: "private_only".to_string(),
        query: "안녕하세요".to_string(),
        top_k: Some(5),
        candidate_k: Some(10),
        record_hits: Some(false),
    };

    let result = service.search(request).await;
    match result {
        Err(elf_service::ServiceError::NonEnglishInput { field }) => {
            assert_eq!(field, "$.query");
        }
        other => panic!("Expected NonEnglishInput, got {other:?}"),
    }
}

async fn build_test_service(dsn: String, qdrant_url: String) -> Option<elf_service::ElfService> {
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
	super::reset_db(&service.db.pool)
		.await
		.expect("Failed to reset test database.");
    Some(service)
}
