use std::sync::{Arc, atomic::AtomicUsize};

use elf_service::{AddEventRequest, EventMessage, NoteOp, Providers, REJECT_EVIDENCE_MISMATCH};

use super::{
	SpyExtractor, StubEmbedding, StubRerank, build_service, test_config, test_db, test_qdrant_url,
};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_invalid_evidence_quote() {
	let Some(test_db) = test_db().await else {
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
	let extractor =
		SpyExtractor { calls: Arc::new(AtomicUsize::new(0)), payload: extractor_payload };
	let providers = Providers::new(
		Arc::new(StubEmbedding { vector_dim: 4_096 }),
		Arc::new(StubRerank),
		Arc::new(extractor),
	);
	let collection = test_db.collection_name("elf_acceptance");
	let cfg = test_config(test_db.dsn().to_string(), qdrant_url, 3, collection);
	let service = build_service(cfg, providers).await.expect("Failed to build service.");

	super::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	let request = AddEventRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: Some("agent_private".to_string()),
		dry_run: Some(false),
		messages: vec![EventMessage {
			role: "user".to_string(),
			content: "This is a message without the expected quote.".to_string(),
			ts: None,
			msg_id: None,
		}],
	};
	let response = service.add_event(request).await.expect("add_event failed.");
	let result = &response.results[0];

	assert_eq!(response.results.len(), 1);
	assert_eq!(result.op, NoteOp::Rejected);
	assert_eq!(result.reason_code.as_deref(), Some(REJECT_EVIDENCE_MISMATCH));

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
