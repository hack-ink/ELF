use std::sync::{Arc, atomic::AtomicUsize};

use crate::acceptance::{SpyExtractor, StubEmbedding, StubRerank};
use elf_domain::memory_policy::MemoryPolicyDecision;
use elf_service::{
	AddEventRequest, EventMessage, NoteOp, Providers, REJECT_EVIDENCE_MISMATCH,
	REJECT_WRITE_POLICY_MISMATCH,
};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_invalid_evidence_quote() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!("Skipping rejects_invalid_evidence_quote; set ELF_PG_DSN to run this test.");

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
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
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let cfg = crate::acceptance::test_config(
		test_db.dsn().to_string(),
		qdrant_url,
		4_096,
		collection,
		docs_collection,
	);
	let service =
		crate::acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	crate::acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	let request = AddEventRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: Some("agent_private".to_string()),
		dry_run: Some(false),
		ingestion_profile: None,
		messages: vec![EventMessage {
			role: "user".to_string(),
			content: "This is a message without the expected quote.".to_string(),
			ts: None,
			msg_id: None,
			write_policy: None,
		}],
	};
	let response = service.add_event(request).await.expect("add_event failed.");
	let result = &response.results[0];

	assert_eq!(response.results.len(), 1);
	assert_eq!(result.op, NoteOp::Rejected);
	assert_eq!(result.reason_code.as_deref(), Some(REJECT_EVIDENCE_MISMATCH));
	assert_eq!(result.policy_decision, MemoryPolicyDecision::Reject);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run."]
async fn rejects_transformed_quote_mismatch_with_write_policy() {
	let Some(test_db) = crate::acceptance::test_db().await else {
		eprintln!(
			"Skipping rejects_transformed_quote_mismatch_with_write_policy; set ELF_PG_DSN to run."
		);

		return;
	};
	let Some(qdrant_url) = crate::acceptance::test_qdrant_url() else {
		eprintln!(
			"Skipping rejects_transformed_quote_mismatch_with_write_policy; set ELF_QDRANT_URL to run."
		);

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
				{ "message_index": 0, "quote": "Alice mentors Bob." }
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
	let docs_collection = test_db.collection_name("elf_acceptance_docs");
	let cfg = crate::acceptance::test_config(
		test_db.dsn().to_string(),
		qdrant_url,
		4_096,
		collection,
		docs_collection,
	);
	let service =
		crate::acceptance::build_service(cfg, providers).await.expect("Failed to build service.");

	crate::acceptance::reset_db(&service.db.pool).await.expect("Failed to reset test database.");

	let request = AddEventRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: Some("agent_private".to_string()),
		dry_run: Some(false),
		ingestion_profile: None,
		messages: vec![EventMessage {
			role: "user".to_string(),
			content: "Alice mentors Bob.".to_string(),
			ts: None,
			msg_id: None,
			write_policy: Some(
				serde_json::from_value(
					serde_json::json!({ "redactions": [{ "kind": "remove", "span": { "start": 0, "end": 5 } }] }),
				)
				.expect("Failed to build write_policy."),
			),
		}],
	};
	let response = service.add_event(request).await.expect("add_event failed.");
	let result = &response.results[0];

	assert_eq!(response.results.len(), 1);
	assert_eq!(result.op, NoteOp::Rejected);
	assert_eq!(result.reason_code.as_deref(), Some(REJECT_WRITE_POLICY_MISMATCH));
	assert_eq!(result.policy_decision, MemoryPolicyDecision::Reject);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
