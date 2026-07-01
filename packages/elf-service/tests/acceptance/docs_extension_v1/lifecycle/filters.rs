use std::collections::HashSet;

use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use crate::acceptance::docs_extension_v1;
use elf_service::{DocsSearchL0Request, Error};

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
