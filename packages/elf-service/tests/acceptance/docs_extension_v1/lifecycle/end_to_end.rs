use std::time::Duration;

use crate::acceptance::docs_extension_v1::{self, DocsContext};

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
			Duration::from_secs(15)
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
