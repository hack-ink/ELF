use std::{sync::Arc, time::Duration};

use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::{sync::oneshot::Sender, task::JoinHandle};

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

fn configure_recency_bias_settings(service: &mut ElfService) {
	service.providers.embedding = Arc::new(NonZeroSearchEmbedding);
	service.cfg.ranking.tie_breaker_weight = 1_000.0;
	service.cfg.ranking.recency_tau_days = 36_500.0;
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
