mod helpers;

pub(crate) use helpers::{
	AGENT_ID, PROJECT_ID, TENANT_ID, assert_first_rebuild, insert_rebuild_sources,
	knowledge_foundation_request, setup_service,
};

use time::OffsetDateTime;

use elf_domain::knowledge::KnowledgePageKind;
use elf_service::{KnowledgePageLintRequest, KnowledgePageSearchRequest};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run this test."]
async fn rebuilds_pages_with_citations_and_detects_stale_sources() {
	let Some(fixture) =
		setup_service("rebuilds_pages_with_citations_and_detects_stale_sources").await
	else {
		return;
	};
	let service = &fixture.service;
	let source_ids = insert_rebuild_sources(service).await;
	let first = service
		.knowledge_page_rebuild(knowledge_foundation_request(source_ids))
		.await
		.expect("knowledge page should rebuild");

	assert_first_rebuild(&first);

	let second = service
		.knowledge_page_rebuild(knowledge_foundation_request(source_ids))
		.await
		.expect("knowledge page should rebuild deterministically");

	assert_eq!(first.page.page.page_id, second.page.page.page_id);
	assert_eq!(first.page.page.rebuild_source_hash, second.page.page.rebuild_source_hash);
	assert_eq!(first.page.page.content_hash, second.page.page.content_hash);

	let second_diff = second
		.page
		.page
		.previous_version_diff
		.as_ref()
		.expect("second rebuild should expose previous-version diff");

	assert_eq!(second_diff["schema"], "elf.knowledge_page.version_diff/v1");
	assert_eq!(second_diff["available"], true);
	assert_eq!(second_diff["source_mutation_allowed"], false);
	assert_eq!(second_diff["content_changed"], false);

	sqlx::query(
		"\
UPDATE memory_notes
SET text = $1, updated_at = $2
WHERE note_id = $3",
	)
	.bind("Fact: Derived knowledge pages changed after the page snapshot was rebuilt.")
	.bind(OffsetDateTime::now_utc())
	.bind(source_ids.note_id)
	.execute(&service.db.pool)
	.await
	.expect("source note should update");

	let lint = service
		.knowledge_page_lint(KnowledgePageLintRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			page_id: first.page.page.page_id,
		})
		.await
		.expect("knowledge page lint should run");

	assert!(lint.findings.iter().any(|finding| {
		finding.finding_type == "stale_source_ref"
			&& finding.source_kind.as_deref() == Some("note")
			&& finding.source_id == Some(source_ids.note_id)
	}));
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL to run this test."]
async fn knowledge_page_search_suppresses_deleted_source_library_spans() {
	let Some(fixture) =
		setup_service("knowledge_page_search_suppresses_deleted_source_library_spans").await
	else {
		return;
	};
	let service = &fixture.service;
	let source_ids = insert_rebuild_sources(service).await;
	let page = service
		.knowledge_page_rebuild(knowledge_foundation_request(source_ids))
		.await
		.expect("knowledge page should rebuild");
	let before_delete = service
		.knowledge_pages_search(KnowledgePageSearchRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: AGENT_ID.to_string(),
			read_profile: "private_plus_project".to_string(),
			query: "Source Library spans".to_string(),
			page_kind: Some(KnowledgePageKind::Project),
			limit: Some(10),
		})
		.await
		.expect("knowledge page search should run");

	assert!(
		before_delete.items.iter().any(|item| item.page_id == page.page.page.page_id
			&& item.source_refs.iter().any(|source_ref| {
				source_ref.source_kind == "doc" || source_ref.source_kind == "doc_chunk"
			})),
		"expected search to return the Source Library-backed page section before delete"
	);

	let private_only = service
		.knowledge_pages_search(KnowledgePageSearchRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: AGENT_ID.to_string(),
			read_profile: "private_only".to_string(),
			query: "Source Library spans".to_string(),
			page_kind: Some(KnowledgePageKind::Project),
			limit: Some(10),
		})
		.await
		.expect("knowledge page search should run");

	assert!(
		private_only.items.iter().all(|item| {
			!item.source_refs.iter().any(|source_ref| {
				source_ref.source_kind == "doc" || source_ref.source_kind == "doc_chunk"
			})
		}),
		"private_only search must not recall project-shared Source Library snippets"
	);

	let ungranted_shared_reader = service
		.knowledge_pages_search(KnowledgePageSearchRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: "agent_without_source_grant".to_string(),
			read_profile: "private_plus_project".to_string(),
			query: "Source Library spans".to_string(),
			page_kind: Some(KnowledgePageKind::Project),
			limit: Some(10),
		})
		.await
		.expect("knowledge page search should run");

	assert!(
		ungranted_shared_reader.items.iter().all(|item| {
			!item.source_refs.iter().any(|source_ref| {
				source_ref.source_kind == "doc" || source_ref.source_kind == "doc_chunk"
			})
		}),
		"project-shared Source Library snippets require an owner or active shared grant"
	);

	sqlx::query("UPDATE doc_documents SET status = 'deleted', updated_at = $1 WHERE doc_id = $2")
		.bind(OffsetDateTime::now_utc())
		.bind(source_ids.doc_id)
		.execute(&service.db.pool)
		.await
		.expect("source document should be marked deleted");

	let after_delete = service
		.knowledge_pages_search(KnowledgePageSearchRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: AGENT_ID.to_string(),
			read_profile: "private_plus_project".to_string(),
			query: "Source Library spans".to_string(),
			page_kind: Some(KnowledgePageKind::Project),
			limit: Some(10),
		})
		.await
		.expect("knowledge page search should run");

	assert!(
		after_delete.items.iter().all(|item| {
			!item.source_refs.iter().any(|source_ref| {
				source_ref.source_kind == "doc" || source_ref.source_kind == "doc_chunk"
			})
		}),
		"deleted Source Library docs and chunks must not be recalled through derived page search"
	);
}
