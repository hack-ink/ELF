use serde_json::Value;
use uuid::Uuid;

use crate::acceptance::docs_extension_v1::{self, DocsContext};
use elf_service::{
	AddNoteInput, AddNoteRequest, DocsExcerptsGetRequest, DocsSearchL0Request, ElfService,
	PayloadLevel, SearchRequest,
};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_search_l0_returns_pointer_and_explain_trajectory() {
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

	let results = service
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
			explain: Some(true),
		})
		.await
		.expect("Failed to search docs.");

	assert_eq!(
		results.trajectory.as_ref().map(|trajectory| trajectory.schema.as_str()),
		Some("doc_retrieval_trajectory/v1")
	);
	assert!(results.trajectory.is_some());
	assert!(!results.items.is_empty());
	assert!(results.items[0].pointer.schema == "source_ref/v1");
	assert!(!results.items[0].pointer.reference.doc_id.is_nil());
	assert!(!results.items[0].pointer.reference.chunk_id.is_nil());
	assert_eq!(
		results.items[0].pointer.reference.source_record_id,
		results.items[0].pointer.reference.doc_id
	);
	assert_eq!(
		results.items[0].pointer.reference.source_span_id,
		results.items[0].pointer.locator.span_id
	);
	assert_eq!(results.items[0].pointer.resolver, "elf_doc_ext/v1");
	assert!(!results.items[0].pointer.locator.span_id.is_nil());
	assert!(!results.trace_id.is_nil());

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run."]
async fn docs_search_l0_note_pointer_roundtrip_hydrates_doc() {
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

	let (source_ref, source_ref_doc_id, source_ref_chunk_id) =
		fetch_docs_pointer_source_ref(&service).await;
	let note_id = add_note_with_pointer_source_ref(&service, source_ref.clone()).await;

	assert!(
		docs_extension_v1::wait_for_note_outbox_done(
			&service.db.pool,
			note_id,
			std::time::Duration::from_secs(15)
		)
		.await,
		"Expected note outbox to reach DONE."
	);

	let search_results = service
		.search_raw_quick(SearchRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "agent".to_string(),
			token_id: None,
			read_profile: "private_only".to_string(),
			payload_level: PayloadLevel::L2,
			query: "peregrine".to_string(),
			top_k: Some(5),
			candidate_k: Some(20),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.expect("Failed to search note with doc pointer source_ref.");
	let has_pointer_source_ref =
		search_results.items.into_iter().any(|item| item.source_ref == source_ref);

	assert!(
		has_pointer_source_ref,
		"Expected search result to include note with pointer source_ref."
	);

	let excerpt = service
		.docs_excerpts_get(DocsExcerptsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "reader".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id: source_ref_doc_id,
			level: "L1".to_string(),
			chunk_id: Some(source_ref_chunk_id),
			quote: None,
			position: None,
			explain: None,
		})
		.await
		.expect("Failed to hydrate excerpt from pointer source_ref.");

	assert!(excerpt.verification.verified);

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

async fn fetch_docs_pointer_source_ref(service: &ElfService) -> (Value, Uuid, Uuid) {
	let search = service
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
		.expect("Failed to search docs for source_ref pointer.");

	assert!(!search.items.is_empty(), "Expected docs_search_l0 to return source_ref pointer.");

	let pointer = search.items[0].pointer.clone();
	let source_ref =
		serde_json::to_value(&pointer).expect("Failed to serialize docs_search_l0 pointer.");

	(source_ref, pointer.reference.doc_id, pointer.reference.chunk_id)
}

async fn add_note_with_pointer_source_ref(service: &ElfService, source_ref: Value) -> Uuid {
	let note = service
		.add_note(AddNoteRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "agent".to_string(),
			scope: "agent_private".to_string(),
			notes: vec![AddNoteInput {
				r#type: "fact".to_string(),
				key: Some("doc_pointer_note".to_string()),
				text: "Peregrine note for source_ref hydration check.".to_string(),
				structured: None,
				importance: 0.5,
				confidence: 0.9,
				ttl_days: None,
				source_ref,
				write_policy: None,
			}],
		})
		.await
		.expect("Failed to add note from docs pointer.");

	note.results[0].note_id.expect("Expected note_id in add_note result.")
}
