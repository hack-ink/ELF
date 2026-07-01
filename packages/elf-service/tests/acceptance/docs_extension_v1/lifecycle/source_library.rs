use serde_json::Value;

use crate::acceptance::docs_extension_v1::{self, DocsContext};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_put_source_library_records_do_not_create_memory_notes() {
	let Some(ctx) = docs_extension_v1::setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory_notes")
		.fetch_one(&service.db.pool)
		.await
		.expect("Failed to count notes before docs_put.");
	let put = docs_extension_v1::put_test_doc_with(
		&service,
		"owner",
		"project_shared",
		Some("chat"),
		"Captured thread",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "chat",
			"ts": "2026-02-25T12:00:00Z",
			"thread_id": "thread-source-library-1",
			"role": "user",
			"source_kind": "social_thread",
			"canonical_uri": "https://example.com/thread/source-library-1",
			"captured_at": "2026-02-25T12:10:00Z",
			"source_created_at": "2026-02-25T11:55:00Z",
			"trust_label": "public_web",
			"author": "Example Researcher",
			"handle": "example-researcher",
			"excerpt_locator": {
				"quote": {
					"exact": "Source libraries should preserve thread context."
				}
			}
		}),
		"Source libraries should preserve thread context. Agents can later promote only selected facts.",
	)
	.await;
	let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory_notes")
		.fetch_one(&service.db.pool)
		.await
		.expect("Failed to count notes after docs_put.");
	let doc_exists: bool =
		sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM doc_documents WHERE doc_id = $1)")
			.bind(put.doc_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("Failed to verify doc row.");
	let stored_source_ref: Value =
		sqlx::query_scalar("SELECT source_ref FROM doc_documents WHERE doc_id = $1")
			.bind(put.doc_id)
			.fetch_one(&service.db.pool)
			.await
			.expect("Failed to fetch normalized source_ref.");

	assert!(doc_exists);
	assert_eq!(after, before, "docs_put must not create durable Memory Notes.");
	assert_eq!(put.source_capture.schema, "doc_source_capture/v1");
	assert_eq!(put.source_capture.source_record_id, put.doc_id);
	assert_eq!(put.source_capture.origin, "https://example.com/thread/source-library-1");
	assert_eq!(put.source_capture.source_type, "social_thread");
	assert_eq!(put.source_capture.visibility_scope, "project_shared");
	assert!(!put.source_capture.source_spans.is_empty());
	assert_eq!(put.source_capture.source_spans[0].schema, "doc_source_span/v1");
	assert_eq!(put.source_capture.source_spans[0].status, "captured");
	assert_eq!(put.source_capture.source_spans[0].reason_code, None);
	assert_eq!(stored_source_ref["source_record_id"], put.doc_id.to_string());
	assert_eq!(stored_source_ref["origin"], "https://example.com/thread/source-library-1");
	assert_eq!(stored_source_ref["source_type"], "social_thread");
	assert_eq!(stored_source_ref["content_hash"], put.content_hash);
	assert!(stored_source_ref["source_spans"].as_array().is_some_and(|spans| !spans.is_empty()));

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
