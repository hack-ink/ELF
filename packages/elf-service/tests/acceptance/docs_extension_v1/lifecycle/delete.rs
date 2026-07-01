use std::time::Duration;

use crate::acceptance::docs_extension_v1::{self, DocsContext};
use elf_service::{DocsDeleteRequest, DocsGetRequest, DocsSearchL0Request, Error, NoteOp};

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
			Duration::from_secs(15)
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
			Duration::from_secs(15)
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
