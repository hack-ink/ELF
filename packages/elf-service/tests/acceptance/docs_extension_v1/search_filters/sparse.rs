use std::time::Duration;

use serde_json::Value;

use crate::acceptance::docs_extension_v1::{self, DocsContext};
use elf_service::DocsSearchL0Request;

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
