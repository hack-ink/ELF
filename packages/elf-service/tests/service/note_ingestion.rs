use std::sync::Arc;

use sqlx::PgPool;

use crate::service::{
	config,
	providers::{DummyEmbedding, DummyRerank, SpyExtractor},
};
use elf_service::{AddNoteInput, AddNoteRequest, ElfService, Error, Providers};
use elf_storage::{db::Db, qdrant::QdrantStore};

fn build_service_with_spy(spy: Arc<SpyExtractor>) -> ElfService {
	let cfg = config::test_config();
	let pool =
		PgPool::connect_lazy(&cfg.storage.postgres.dsn).expect("Failed to create lazy pool.");
	let db = Db { pool };
	let qdrant = QdrantStore::new(&cfg.storage.qdrant).expect("Failed to create Qdrant store.");
	let providers = Providers::new(Arc::new(DummyEmbedding), Arc::new(DummyRerank), spy);

	ElfService::with_providers(cfg, db, qdrant, providers)
}

#[tokio::test]
async fn add_note_does_not_call_llm() {
	let cfg = config::test_config();
	let spy = Arc::new(SpyExtractor::new());
	let service = build_service_with_spy(spy.clone());
	let req = AddNoteRequest {
		tenant_id: "t1".to_string(),
		project_id: "p1".to_string(),
		agent_id: "a1".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![AddNoteInput {
			r#type: "fact".to_string(),
			key: None,
			text: "こんにちは".to_string(),
			structured: None,
			importance: 0.5,
			confidence: 0.5,
			ttl_days: None,
			source_ref: serde_json::json!({}),
			write_policy: None,
		}],
	};
	let result = service.add_note(req).await;

	assert!(cfg.security.reject_non_english);
	assert!(matches!(result, Err(Error::NonEnglishInput { .. })));
	assert_eq!(spy.count(), 0);
}

#[tokio::test]
async fn add_note_rejects_empty_notes() {
	let spy = Arc::new(SpyExtractor::new());
	let service = build_service_with_spy(spy.clone());
	let req = AddNoteRequest {
		tenant_id: "t1".to_string(),
		project_id: "p1".to_string(),
		agent_id: "a1".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![],
	};
	let result = service.add_note(req).await;

	assert!(matches!(result, Err(Error::InvalidRequest { .. })));
	assert_eq!(spy.count(), 0);
}
