use std::time::Duration;

use crate::acceptance::docs_extension_v1::{self, DocsContext};
use elf_service::{DocsExcerptsGetRequest, DocsPutRequest};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."]
async fn docs_put_applies_write_policy_and_excerpt_by_chunk_id_is_verified() {
	let Some(ctx) = docs_extension_v1::setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let content = "Alpha normal text then secret sk-abcdef and trailing content.";
	let secret = "sk-abcdef";
	let start = content.find(secret).expect("Expected secret in content.");
	let end = start + secret.len();
	let write_policy = serde_json::from_value(serde_json::json!({
		"exclusions": [{"start": start, "end": end}],
	}))
	.expect("Failed to build write_policy.");
	let put = service
		.docs_put(DocsPutRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "owner".to_string(),
			scope: "project_shared".to_string(),
			doc_type: None,
			title: Some("Docs write_policy sample".to_string()),
			write_policy: Some(write_policy),
			source_ref: serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "knowledge",
				"ts": "2026-02-25T12:00:00Z",
			}),
			content: content.to_string(),
		})
		.await
		.expect("Failed to put doc with write policy.");
	let (handle, shutdown) = docs_extension_v1::spawn_doc_worker(&service).await;

	assert!(
		docs_extension_v1::wait_for_doc_outbox_done(
			&service.db.pool,
			put.doc_id,
			Duration::from_secs(15)
		)
		.await,
		"Expected doc outbox to reach DONE."
	);

	let chunk_id = docs_extension_v1::fetch_first_doc_chunk_id(&service, put.doc_id)
		.await
		.expect("Expected chunk id from transformed doc.");
	let excerpt = service
		.docs_excerpts_get(DocsExcerptsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "reader".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id: put.doc_id,
			level: "L1".to_string(),
			chunk_id: Some(chunk_id),
			quote: None,
			position: None,
			explain: None,
		})
		.await
		.expect("Failed to hydrate excerpt by chunk_id.");

	assert!(excerpt.verification.verified);
	assert!(!excerpt.excerpt.is_empty());
	assert!(!excerpt.excerpt.contains(secret));
	assert!(!excerpt.locator.span_id.is_nil());

	let captured_chunk_span = put
		.source_capture
		.source_spans
		.iter()
		.find(|span| span.chunk_id == Some(chunk_id))
		.expect("Expected captured source span for hydrated chunk.");

	assert_eq!(excerpt.locator.span_id, captured_chunk_span.span_id);
	assert_eq!(excerpt.verification.content_hash, put.content_hash);
	assert!(put.write_policy_audit.is_some());
	assert_eq!(put.source_capture.policy_spans.len(), 1);
	assert_eq!(put.source_capture.policy_spans[0].status, "excluded");
	assert_eq!(
		put.source_capture.policy_spans[0].reason_code.as_deref(),
		Some("WRITE_POLICY_EXCLUSION")
	);

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
