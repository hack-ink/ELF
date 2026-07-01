use std::collections::HashSet;

use crate::acceptance::docs_extension_v1;
use elf_service::DocsSearchL0Request;

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_search_l0_respects_doc_ts_filter() {
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
	let doc_ts_windowed_results = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: "reader".to_string(),
			scope: Some("project_shared".to_string()),
			status: None,
			doc_type: None,
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: Some("2026-01-01T00:00:00Z".to_string()),
			ts_lte: Some("2026-12-31T23:59:59Z".to_string()),
			read_profile: "all_scopes".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(20),
			candidate_k: Some(50),
			explain: None,
		})
		.await
		.expect("Failed to search docs by doc_ts range.");
	let doc_ts_windowed_ids =
		doc_ts_windowed_results.items.into_iter().map(|item| item.doc_id).collect::<HashSet<_>>();

	assert!(doc_ts_windowed_ids.contains(&shared_knowledge_doc));
	assert!(!doc_ts_windowed_ids.contains(&older_shared_knowledge_doc));
	assert!(!doc_ts_windowed_ids.contains(&private_chat_doc));

	docs_extension_v1::cleanup_docs_filter_fixture(test_db, handle, shutdown).await;
}
