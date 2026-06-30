use crate::acceptance::docs_extension_v1::{self, DocsContext, TEST_CONTENT, payload_string};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_search_l0_requires_qdrant_payload_indexes_for_filters() {
	let Some(ctx) = docs_extension_v1::setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let doc = docs_extension_v1::put_test_doc(&service).await;
	let (handle, shutdown) = docs_extension_v1::spawn_doc_worker(&service).await;

	assert!(
		docs_extension_v1::wait_for_doc_outbox_done(
			&service.db.pool,
			doc.doc_id,
			std::time::Duration::from_secs(15)
		)
		.await,
		"Expected doc outbox to reach DONE."
	);

	docs_extension_v1::verify_docs_qdrant_filter_indexes(&service).await;

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_search_l0_projects_source_ref_payload_fields() {
	let Some(ctx) = docs_extension_v1::setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let source_ts = "2025-01-01T10:00:00Z";
	let cases = [
		(
			"chat",
			"Docs chat source ref sample",
			serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "chat",
				"ts": source_ts,
				"thread_id": "thread-42",
				"role": "assistant"
			}),
			("thread_id", "thread-42"),
			["domain", "repo"],
		),
		(
			"search",
			"Docs search source ref sample",
			serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "search",
				"ts": source_ts,
				"query": "What is payload indexing?",
				"url": "https://docs.example.com/search",
				"domain": "docs.example.com",
				"provider": "web"
			}),
			("domain", "docs.example.com"),
			["thread_id", "repo"],
		),
		(
			"dev",
			"Docs dev source ref sample",
			serde_json::json!({
				"schema": "doc_source_ref/v1",
				"doc_type": "dev",
				"ts": source_ts,
				"repo": "elf-org/docs",
				"commit_sha": "9f0a3f4c4eb58bfcf4a5f4f9d0c7be0e13c2f8d19"
			}),
			("repo", "elf-org/docs"),
			["thread_id", "domain"],
		),
	];
	let mut docs = Vec::new();

	for (doc_type, title, source_ref, expected_present, expected_absent) in cases {
		let doc = docs_extension_v1::put_test_doc_with(
			&service,
			"owner",
			"project_shared",
			Some(doc_type),
			title,
			source_ref,
			TEST_CONTENT,
		)
		.await;

		docs.push((doc.doc_id, expected_present, expected_absent));
	}

	let (handle, shutdown) = docs_extension_v1::spawn_doc_worker(&service).await;

	for (doc_id, expected_present, expected_absent) in &docs {
		assert!(
			docs_extension_v1::wait_for_doc_outbox_done(
				&service.db.pool,
				*doc_id,
				std::time::Duration::from_secs(15)
			)
			.await,
			"Expected doc outbox to reach DONE."
		);

		let point = docs_extension_v1::fetch_first_doc_chunk_point(&service, *doc_id)
			.await
			.expect("Expected doc chunk point in Qdrant.");

		assert_eq!(point.payload.get("doc_ts").and_then(payload_string), Some(source_ts));
		assert_eq!(
			point.payload.get(expected_present.0).and_then(payload_string),
			Some(expected_present.1)
		);

		for key in expected_absent {
			assert!(!point.payload.contains_key(*key));
		}
	}

	_ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
