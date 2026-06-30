use crate::acceptance::docs_extension_v1::{self, DocsContext};
use elf_service::{DocsExcerptsGetRequest, TextQuoteSelector};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_excerpts_get_supports_l0_and_returns_locator_and_optional_trajectory() {
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

	let excerpt = service
		.docs_excerpts_get(DocsExcerptsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "reader".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id: doc.doc_id,
			level: "L0".to_string(),
			chunk_id: None,
			quote: Some(TextQuoteSelector {
				exact: "Keyword: peregrine.".to_string(),
				prefix: Some("evidence. ".to_string()),
				suffix: Some("\nSecond".to_string()),
			}),
			position: None,
			explain: Some(true),
		})
		.await
		.expect("Failed to hydrate excerpt.");

	assert_eq!(excerpt.locator.selector_kind, "quote");
	assert!(excerpt.locator.match_end_offset > excerpt.locator.match_start_offset);
	assert!(!excerpt.locator.span_id.is_nil());
	assert!(excerpt.excerpt.len() <= 256);
	assert!(excerpt.trajectory.is_some());
	assert_eq!(
		excerpt.trajectory.as_ref().map(|trajectory| trajectory.schema.as_str()),
		Some("doc_retrieval_trajectory/v1")
	);
	assert!(!excerpt.trace_id.is_nil());

	let no_explain = service
		.docs_excerpts_get(DocsExcerptsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "reader".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id: doc.doc_id,
			level: "L0".to_string(),
			chunk_id: None,
			quote: Some(TextQuoteSelector {
				exact: "Keyword: peregrine.".to_string(),
				prefix: Some("evidence. ".to_string()),
				suffix: Some("\nSecond".to_string()),
			}),
			position: None,
			explain: Some(false),
		})
		.await
		.expect("Failed to hydrate excerpt.");

	assert!(no_explain.trajectory.is_none());

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
