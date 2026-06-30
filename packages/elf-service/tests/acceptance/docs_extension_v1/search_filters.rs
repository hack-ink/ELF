use std::{collections::HashSet, sync::Arc, time::Duration};

use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::{sync::oneshot::Sender, task::JoinHandle};
use uuid::Uuid;

use crate::acceptance::docs_extension_v1::{self, DocsContext, TEST_CONTENT};
use elf_config::EmbeddingProviderConfig;
use elf_service::{BoxFuture, DocsSearchL0Request, ElfService, EmbeddingProvider, Result};

struct NonZeroSearchEmbedding;
impl EmbeddingProvider for NonZeroSearchEmbedding {
	fn embed<'a>(
		&'a self,
		cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, Result<Vec<Vec<f32>>>> {
		let vector = vec![0.1_f32; cfg.dimensions as usize];

		Box::pin(async move { Ok(vec![vector; texts.len()]) })
	}
}

struct DocsFilterFixtureIds {
	search_domain_doc_id: Uuid,
	search_other_domain_doc_id: Uuid,
	repo_doc_id: Uuid,
	repo_other_doc_id: Uuid,
}

fn configure_recency_bias_settings(service: &mut ElfService) {
	service.providers.embedding = Arc::new(NonZeroSearchEmbedding);
	service.cfg.ranking.tie_breaker_weight = 1_000.0;
	service.cfg.ranking.recency_tau_days = 36_500.0;
}

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

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."]
async fn docs_search_l0_sparse_mode_records_expected_vector_search_channels() {
	let Some(ctx) = docs_extension_v1::setup_docs_context().await else { return };
	let DocsContext { test_db, service } = ctx;
	let doc = docs_extension_v1::put_test_doc(&service).await;
	let (handle, shutdown) = docs_extension_v1::spawn_doc_worker(&service).await;

	assert!(
		docs_extension_v1::wait_for_doc_outbox_done(
			&service.db.pool,
			doc.doc_id,
			Duration::from_secs(15),
		)
		.await,
		"Expected doc outbox to reach DONE."
	);

	let cases = [
		("off", vec!["dense"]),
		("on", vec!["dense", "sparse"]),
		("auto", vec!["dense", "sparse"]),
	];

	for (sparse_mode, expected_channels) in cases {
		let response = service
			.docs_search_l0(DocsSearchL0Request {
				tenant_id: "t".to_string(),
				project_id: "p".to_string(),
				caller_agent_id: "reader".to_string(),
				scope: None,
				status: None,
				doc_type: None,
				sparse_mode: Some(sparse_mode.to_string()),
				domain: None,
				repo: None,
				agent_id: None,
				thread_id: None,
				updated_after: None,
				updated_before: None,
				ts_gte: None,
				ts_lte: None,
				read_profile: "private_plus_project".to_string(),
				query: "https://elf.example/docs?query=peregrine".to_string(),
				top_k: Some(20),
				candidate_k: Some(50),
				explain: Some(true),
			})
			.await
			.expect("Failed to search docs with sparse_mode set.");
		let trajectory = response.trajectory.as_ref().expect("Expected explain trajectory.");
		let vector_search_stats =
			docs_extension_v1::trajectory_stage_stats(trajectory, "vector_search")
				.expect("Expected vector_search stage in trajectory.");
		let vector_search_channels = vector_search_stats
			.get("channels")
			.and_then(Value::as_array)
			.expect("Expected vector_search stats channels.");
		let observed_channels = vector_search_channels
			.iter()
			.map(|channel| channel.as_str().expect("Expected channel string.").to_string())
			.collect::<Vec<_>>();

		assert_eq!(observed_channels, expected_channels);
	}

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
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

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_URL (or ELF_QDRANT_GRPC_URL) to run this test."]
async fn docs_search_l0_recency_bias_orders_newer_doc_first_and_records_projection_signals() {
	let Some(ctx) = docs_extension_v1::setup_docs_context().await else { return };
	let DocsContext { test_db, mut service } = ctx;

	configure_recency_bias_settings(&mut service);

	let (handle, shutdown) = seed_recency_bias_docs_for_search(&service).await;

	assert_docs_search_l0_recency_projection(&service).await;

	let _ = shutdown.send(());

	handle.abort();

	let _ = handle.await;

	drop(service);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

async fn seed_recency_bias_docs_for_search(service: &ElfService) -> (JoinHandle<()>, Sender<()>) {
	let newer_doc = docs_extension_v1::put_test_doc_with(
		service,
		"owner",
		"project_shared",
		Some("knowledge"),
		"Recency newer doc",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-27T12:00:00Z",
		}),
		TEST_CONTENT,
	)
	.await;
	let older_doc = docs_extension_v1::put_test_doc_with(
		service,
		"owner",
		"project_shared",
		Some("knowledge"),
		"Recency older doc",
		serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-20T12:00:00Z",
		}),
		TEST_CONTENT,
	)
	.await;
	let (handle, shutdown) = docs_extension_v1::spawn_doc_worker(service).await;

	assert!(
		docs_extension_v1::wait_for_doc_outbox_done(
			&service.db.pool,
			newer_doc.doc_id,
			Duration::from_secs(15),
		)
		.await,
		"Expected newer doc outbox to reach DONE."
	);
	assert!(
		docs_extension_v1::wait_for_doc_outbox_done(
			&service.db.pool,
			older_doc.doc_id,
			Duration::from_secs(15),
		)
		.await,
		"Expected older doc outbox to reach DONE."
	);

	let older_ts = OffsetDateTime::parse("2020-01-01T00:00:00Z", &Rfc3339)
		.expect("Failed to parse older doc timestamp.");

	sqlx::query("UPDATE doc_documents SET updated_at = $1 WHERE doc_id = $2")
		.bind(older_ts)
		.bind(older_doc.doc_id)
		.execute(&service.db.pool)
		.await
		.expect("Failed to set deterministic updated_at for older doc.");

	(handle, shutdown)
}

async fn assert_docs_search_l0_recency_projection(service: &ElfService) {
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
			top_k: Some(2),
			candidate_k: Some(20),
			explain: Some(true),
		})
		.await
		.expect("Failed to search docs for recency ordering.");
	let ordered_ids = results.items.iter().map(|item| item.doc_id).collect::<Vec<_>>();

	assert!(ordered_ids.len() >= 2);

	let newest_id = results
		.items
		.iter()
		.max_by_key(|item| item.updated_at.unix_timestamp())
		.expect("Expected returned item.")
		.doc_id;

	assert_eq!(results.items[0].doc_id, newest_id);
	assert!(results.items[0].updated_at > results.items[1].updated_at);

	let trajectory = results.trajectory.as_ref().expect("Expected explain trajectory.");
	let result_projection =
		docs_extension_v1::trajectory_stage_stats(trajectory, "result_projection")
			.expect("Expected result_projection stage in trajectory.");

	assert!(result_projection.get("pre_authorization_candidates").is_some());
	assert!(result_projection.get("returned_items").is_some());
	assert!(result_projection.get("recency_tau_days").is_some());
	assert!(result_projection.get("tie_breaker_weight").is_some());
	assert_eq!(result_projection.get("recency_boost_applied"), Some(&Value::Bool(true)));
}
