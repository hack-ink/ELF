use std::{collections::HashSet, time::Duration};

use uuid::Uuid;

use crate::acceptance::docs_extension_v1::{self, DocsContext, TEST_CONTENT};
use elf_service::DocsSearchL0Request;

struct DocsFilterFixtureIds {
	search_domain_doc_id: Uuid,
	search_other_domain_doc_id: Uuid,
	repo_doc_id: Uuid,
	repo_other_doc_id: Uuid,
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."]
async fn docs_search_l0_filters_include_and_exclude_by_doc_type_and_domain_or_repo() {
	let Some(ctx) = docs_extension_v1::setup_docs_context().await else { return };
	let docs = seed_docs_filter_fixtures(&ctx).await;
	let DocsContext { test_db, service } = ctx;
	let (handle, shutdown) = docs_extension_v1::spawn_doc_worker(&service).await;

	for doc_id in [
		docs.search_domain_doc_id,
		docs.search_other_domain_doc_id,
		docs.repo_doc_id,
		docs.repo_other_doc_id,
	]
	.iter()
	{
		assert!(
			docs_extension_v1::wait_for_doc_outbox_done(
				&service.db.pool,
				*doc_id,
				Duration::from_secs(15),
			)
			.await,
			"Expected docs outbox to reach DONE."
		);
	}

	let search_domain_results = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: "reader".to_string(),
			scope: Some("project_shared".to_string()),
			status: None,
			doc_type: Some("search".to_string()),
			sparse_mode: None,
			domain: Some("docs.example.com".to_string()),
			repo: None,
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			read_profile: "all_scopes".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(20),
			candidate_k: Some(50),
			explain: None,
		})
		.await
		.expect("Failed to search docs by domain.");
	let search_domain_result_ids =
		search_domain_results.items.into_iter().map(|item| item.doc_id).collect::<HashSet<_>>();

	assert!(search_domain_result_ids.contains(&docs.search_domain_doc_id));
	assert!(!search_domain_result_ids.contains(&docs.search_other_domain_doc_id));
	assert!(!search_domain_result_ids.contains(&docs.repo_doc_id));
	assert!(!search_domain_result_ids.contains(&docs.repo_other_doc_id));

	let repo_results = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: "reader".to_string(),
			scope: Some("project_shared".to_string()),
			status: None,
			doc_type: Some("dev".to_string()),
			sparse_mode: None,
			domain: None,
			repo: Some("elf-org/docs".to_string()),
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			read_profile: "all_scopes".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(20),
			candidate_k: Some(50),
			explain: None,
		})
		.await
		.expect("Failed to search docs by repo.");
	let repo_result_ids =
		repo_results.items.into_iter().map(|item| item.doc_id).collect::<HashSet<_>>();

	assert!(repo_result_ids.contains(&docs.repo_doc_id));
	assert!(!repo_result_ids.contains(&docs.repo_other_doc_id));
	assert!(!repo_result_ids.contains(&docs.search_domain_doc_id));
	assert!(!repo_result_ids.contains(&docs.search_other_domain_doc_id));

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

async fn seed_docs_filter_fixtures(ctx: &DocsContext) -> DocsFilterFixtureIds {
	let search_domain_doc = docs_extension_v1::put_test_doc_with(
		&ctx.service,
		"owner",
		"project_shared",
		Some("search"),
		"Docs domain include sample",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "search",
			"ts": "2026-02-25T12:00:00Z",
			"query": "How to fetch docs",
			"domain": "docs.example.com",
			"url": "https://docs.example.com/guide",
		}),
		TEST_CONTENT,
	)
	.await;
	let search_other_domain_doc = docs_extension_v1::put_test_doc_with(
		&ctx.service,
		"owner",
		"project_shared",
		Some("search"),
		"Docs domain exclude sample",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "search",
			"ts": "2026-02-25T12:00:00Z",
			"query": "How to build",
			"domain": "api.example.org",
			"url": "https://api.example.org/reference",
		}),
		TEST_CONTENT,
	)
	.await;
	let repo_doc = docs_extension_v1::put_test_doc_with(
		&ctx.service,
		"owner",
		"project_shared",
		Some("dev"),
		"Docs repo include sample",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "dev",
			"ts": "2026-02-25T12:00:00Z",
			"repo": "elf-org/docs",
			"commit_sha": "9f0a3f4c4eb58bfcf4a5f4f9d0c7be0e13c2f8d19",
		}),
		TEST_CONTENT,
	)
	.await;
	let repo_other_doc = docs_extension_v1::put_test_doc_with(
		&ctx.service,
		"owner",
		"project_shared",
		Some("dev"),
		"Docs repo exclude sample",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "dev",
			"ts": "2026-02-25T12:00:00Z",
			"repo": "other-org/docs",
			"commit_sha": "4e3d9ec4d2a59a2f6c7d7f3d4c6e8a5b1f7b9d3f",
		}),
		TEST_CONTENT,
	)
	.await;

	DocsFilterFixtureIds {
		search_domain_doc_id: search_domain_doc.doc_id,
		search_other_domain_doc_id: search_other_domain_doc.doc_id,
		repo_doc_id: repo_doc.doc_id,
		repo_other_doc_id: repo_other_doc.doc_id,
	}
}
