use std::{collections::HashSet, time::Duration};

use tokio::{sync::oneshot::Sender, task::JoinHandle};
use uuid::Uuid;

use crate::acceptance::docs_extension_v1::helpers::{
	context::{self, DocsContext, TEST_CONTENT},
	outbox, worker,
};
use elf_service::{DocsSearchL0Request, ElfService};
use elf_testkit::TestDatabase;

pub(crate) async fn create_docs_search_filter_fixture(
	ctx: DocsContext,
) -> (TestDatabase, ElfService, Uuid, Uuid, Uuid, JoinHandle<()>, Sender<()>) {
	let DocsContext { test_db, service } = ctx;
	let shared_knowledge_doc = context::put_test_doc_with(
		&service,
		"owner",
		"project_shared",
		None,
		"Docs filter sample",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
		}),
		TEST_CONTENT,
	)
	.await;
	let older_shared_knowledge_doc = context::put_test_doc_with(
		&service,
		"owner",
		"project_shared",
		None,
		"Docs old filter sample",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2025-01-01T10:00:00Z",
		}),
		TEST_CONTENT,
	)
	.await;
	let private_chat_doc = context::put_test_doc_with(
		&service,
		"assistant",
		"agent_private",
		Some("chat"),
		"Docs chat sample",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "chat",
			"ts": "2026-02-25T12:00:00Z",
			"thread_id": "shared-chat-thread",
			"role": "assistant"
		}),
		TEST_CONTENT,
	)
	.await;
	let (handle, shutdown) = worker::spawn_doc_worker(&service).await;

	assert!(
		outbox::wait_for_doc_outbox_done(
			&service.db.pool,
			shared_knowledge_doc.doc_id,
			Duration::from_secs(15)
		)
		.await,
		"Expected shared docs outbox to reach DONE."
	);
	assert!(
		outbox::wait_for_doc_outbox_done(
			&service.db.pool,
			older_shared_knowledge_doc.doc_id,
			Duration::from_secs(15)
		)
		.await,
		"Expected older shared docs outbox to reach DONE."
	);
	assert!(
		outbox::wait_for_doc_outbox_done(
			&service.db.pool,
			private_chat_doc.doc_id,
			Duration::from_secs(15)
		)
		.await,
		"Expected private docs outbox to reach DONE."
	);

	(
		test_db,
		service,
		shared_knowledge_doc.doc_id,
		older_shared_knowledge_doc.doc_id,
		private_chat_doc.doc_id,
		handle,
		shutdown,
	)
}

pub(crate) async fn cleanup_docs_filter_fixture(
	test_db: TestDatabase,
	_handle: JoinHandle<()>,
	shutdown: Sender<()>,
) {
	let _ = shutdown.send(());

	_handle.abort();

	let _ = _handle.await;

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

pub(crate) async fn search_doc_ids_with_filters(
	service: &ElfService,
	scope: Option<&str>,
	doc_type: Option<&str>,
	agent_id: Option<&str>,
	updated_after: Option<&str>,
	updated_before: Option<&str>,
	caller_agent_id: &str,
) -> HashSet<Uuid> {
	let results = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: caller_agent_id.to_string(),
			scope: scope.map(str::to_string),
			status: None,
			doc_type: doc_type.map(str::to_string),
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: agent_id.map(str::to_string),
			thread_id: None,
			updated_after: updated_after.map(str::to_string),
			updated_before: updated_before.map(str::to_string),
			ts_gte: None,
			ts_lte: None,
			read_profile: "all_scopes".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(20),
			candidate_k: Some(50),
			explain: None,
		})
		.await
		.expect("Failed to search docs.");

	results.items.into_iter().map(|item| item.doc_id).collect()
}
