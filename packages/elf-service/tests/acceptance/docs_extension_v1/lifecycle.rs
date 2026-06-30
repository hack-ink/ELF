use std::collections::HashSet;

use serde_json::Value;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use crate::acceptance::docs_extension_v1::{self, DocsContext};
use elf_service::{
	DocsDeleteRequest, DocsExcerptsGetRequest, DocsGetRequest, DocsPutRequest, DocsSearchL0Request,
	Error, NoteOp,
};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_put_get_excerpts_and_search_l0_work_end_to_end() {
	let Some(ctx) = docs_extension_v1::setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let put = docs_extension_v1::put_test_doc(&service).await;

	docs_extension_v1::assert_doc_get(&service, put.doc_id).await;
	docs_extension_v1::assert_doc_excerpt(&service, put.doc_id, put.content_hash.as_str()).await;

	let (handle, shutdown) = docs_extension_v1::spawn_doc_worker(&service).await;

	assert!(
		docs_extension_v1::wait_for_doc_outbox_done(
			&service.db.pool,
			put.doc_id,
			std::time::Duration::from_secs(15)
		)
		.await,
		"Expected doc outbox to reach DONE."
	);

	docs_extension_v1::assert_docs_search_l0(&service, put.doc_id).await;

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_delete_marks_doc_deleted_and_removes_doc_vectors() {
	let Some(ctx) = docs_extension_v1::setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let put = docs_extension_v1::put_test_doc(&service).await;
	let (handle, shutdown) = docs_extension_v1::spawn_doc_worker(&service).await;

	assert!(
		docs_extension_v1::wait_for_doc_outbox_done(
			&service.db.pool,
			put.doc_id,
			std::time::Duration::from_secs(15)
		)
		.await,
		"Expected doc UPSERT outbox to reach DONE."
	);
	assert!(
		docs_extension_v1::fetch_first_doc_chunk_point(&service, put.doc_id).await.is_some(),
		"Expected indexed doc chunk before delete."
	);

	let deleted = service
		.docs_delete(DocsDeleteRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "owner".to_string(),
			doc_id: put.doc_id,
		})
		.await
		.expect("Failed to delete Source Library doc.");

	assert_eq!(deleted.doc_id, put.doc_id);
	assert_eq!(deleted.op, NoteOp::Delete);
	assert!(deleted.chunk_delete_count > 0);
	assert!(
		docs_extension_v1::wait_for_doc_outbox_done(
			&service.db.pool,
			put.doc_id,
			std::time::Duration::from_secs(15)
		)
		.await,
		"Expected doc DELETE outbox to reach DONE."
	);

	let get_after_delete = service
		.docs_get(DocsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "owner".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id: put.doc_id,
		})
		.await;
	let search_after_delete = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: "reader".to_string(),
			scope: None,
			status: None,
			doc_type: None,
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			read_profile: "private_plus_project".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(5),
			candidate_k: Some(20),
			explain: None,
		})
		.await
		.expect("Failed to search docs after delete.");
	let second_delete = service
		.docs_delete(DocsDeleteRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "owner".to_string(),
			doc_id: put.doc_id,
		})
		.await
		.expect("Second Source Library delete should be idempotent.");

	assert!(matches!(get_after_delete, Err(Error::NotFound { .. })));
	assert!(search_after_delete.items.iter().all(|item| item.doc_id != put.doc_id));
	assert!(
		docs_extension_v1::fetch_first_doc_chunk_point(&service, put.doc_id).await.is_none(),
		"Deleted Source Library doc chunk must be removed from Qdrant docs index."
	);
	assert_eq!(second_delete.op, NoteOp::None);
	assert_eq!(second_delete.chunk_delete_count, 0);

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

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

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_search_l0_respects_scope_doc_type_agent_id_and_updated_after_filters() {
	let Some(ctx) = docs_extension_v1::setup_docs_context().await else { return };
	let (
		test_db,
		service,
		shared_knowledge_doc,
		_older_shared_knowledge_doc,
		private_chat_doc,
		handle,
		shutdown,
	) = docs_extension_v1::create_docs_search_filter_fixture(ctx).await;
	let shared_scope_results = docs_extension_v1::search_doc_ids_with_filters(
		&service,
		Some("project_shared"),
		None,
		None,
		None,
		None,
		"reader",
	)
	.await;

	assert!(shared_scope_results.contains(&shared_knowledge_doc));
	assert!(!shared_scope_results.contains(&private_chat_doc));

	let chat_results = docs_extension_v1::search_doc_ids_with_filters(
		&service,
		None,
		Some("chat"),
		None,
		None,
		None,
		"reader",
	)
	.await;

	assert!(!chat_results.contains(&private_chat_doc));
	assert!(!chat_results.contains(&shared_knowledge_doc));

	let assistant_chat_results = docs_extension_v1::search_doc_ids_with_filters(
		&service,
		None,
		Some("chat"),
		None,
		None,
		None,
		"assistant",
	)
	.await;

	assert!(assistant_chat_results.contains(&private_chat_doc));
	assert!(!assistant_chat_results.contains(&shared_knowledge_doc));

	let assistant_results = docs_extension_v1::search_doc_ids_with_filters(
		&service,
		None,
		None,
		Some("assistant"),
		None,
		None,
		"reader",
	)
	.await;

	assert!(!assistant_results.contains(&private_chat_doc));
	assert!(!assistant_results.contains(&shared_knowledge_doc));

	let past = (OffsetDateTime::now_utc() - Duration::seconds(60))
		.format(&Rfc3339)
		.expect("Failed to format past RFC3339 timestamp.");
	let future = (OffsetDateTime::now_utc() + Duration::seconds(60))
		.format(&Rfc3339)
		.expect("Failed to format future RFC3339 timestamp.");
	let updated_after_past_results = docs_extension_v1::search_doc_ids_with_filters(
		&service,
		None,
		None,
		None,
		Some(&past),
		None,
		"reader",
	)
	.await;

	assert!(updated_after_past_results.contains(&shared_knowledge_doc));
	assert!(!updated_after_past_results.contains(&private_chat_doc));

	let updated_after_future_results = docs_extension_v1::search_doc_ids_with_filters(
		&service,
		None,
		None,
		None,
		Some(&future),
		None,
		"reader",
	)
	.await;

	assert!(updated_after_future_results.is_empty());

	docs_extension_v1::cleanup_docs_filter_fixture(test_db, handle, shutdown).await;
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_search_l0_respects_thread_id_filter_for_chat_docs() {
	let Some(ctx) = docs_extension_v1::setup_docs_context().await else { return };
	let (
		test_db,
		service,
		shared_knowledge_doc,
		older_shared_knowledge_doc,
		private_chat_doc,
		handle,
		shutdown,
	) = docs_extension_v1::create_docs_search_filter_fixture(ctx).await;
	let thread_filter_results = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: "assistant".to_string(),
			scope: None,
			status: None,
			doc_type: Some("chat".to_string()),
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: Some("shared-chat-thread".to_string()),
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			read_profile: "private_plus_project".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(20),
			candidate_k: Some(50),
			explain: None,
		})
		.await
		.expect("Failed to search docs with thread_id filter.");
	let thread_filtered_docs =
		thread_filter_results.items.into_iter().map(|item| item.doc_id).collect::<HashSet<_>>();

	assert!(thread_filtered_docs.contains(&private_chat_doc));
	assert!(!thread_filtered_docs.contains(&shared_knowledge_doc));
	assert!(!thread_filtered_docs.contains(&older_shared_knowledge_doc));

	docs_extension_v1::cleanup_docs_filter_fixture(test_db, handle, shutdown).await;
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."]
async fn docs_search_l0_requires_chat_doc_type_for_thread_id() {
	let Some(ctx) = docs_extension_v1::setup_docs_context().await else { return };
	let (
		test_db,
		service,
		_shared_knowledge_doc,
		_older_shared_knowledge_doc,
		_private_chat_doc,
		handle,
		shutdown,
	) = docs_extension_v1::create_docs_search_filter_fixture(ctx).await;
	let result = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: "assistant".to_string(),
			scope: None,
			status: None,
			doc_type: None,
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: Some("shared-chat-thread".to_string()),
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			read_profile: "private_plus_project".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(20),
			candidate_k: Some(50),
			explain: None,
		})
		.await;

	match result {
		Err(Error::InvalidRequest { message }) => {
			assert!(message.contains("thread_id requires"));
		},
		other => {
			panic!("Expected InvalidRequest for thread_id without chat doc_type, got {other:?}")
		},
	}

	docs_extension_v1::cleanup_docs_filter_fixture(test_db, handle, shutdown).await;
}

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
			std::time::Duration::from_secs(15)
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
