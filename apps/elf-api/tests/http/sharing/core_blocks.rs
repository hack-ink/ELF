use axum::http::StatusCode;
use uuid::Uuid;

use crate::helpers::{self, TEST_AGENT_A, TEST_AGENT_B};
use elf_api::{routes, state::AppState};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn core_blocks_are_explicitly_attached_and_separate_from_archival_search() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else {
		return;
	};
	let config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state.clone());
	let admin_app = routes::admin_router(state.clone());
	let private_block_id = helpers::create_core_block(
		&admin_app,
		"agent_private",
		"private_operating_context",
		"Preference: Keep core context separate from archival search.",
	)
	.await;
	let note_id = Uuid::new_v4();

	helpers::insert_note(
		&state,
		note_id,
		"agent_private",
		TEST_AGENT_A,
		"Fact: This archival note must not appear in attached core blocks.",
	)
	.await;

	let (status, _) =
		helpers::attach_core_block(&admin_app, private_block_id, TEST_AGENT_A, "private_only")
			.await;
	let before_sessions = helpers::search_session_count(&state).await;
	let blocks = helpers::get_core_blocks(&app, TEST_AGENT_A, "private_only").await;
	let after_sessions = helpers::search_session_count(&state).await;

	assert_eq!(status, StatusCode::OK);
	assert_eq!(before_sessions, after_sessions);
	assert_eq!(blocks["schema"], "elf.core_memory_blocks/v1");
	assert_eq!(blocks["items"].as_array().expect("items array").len(), 1);
	assert_eq!(
		blocks["items"][0]["content"],
		"Preference: Keep core context separate from archival search."
	);
	assert_eq!(blocks["items"][0]["source_ref"]["schema"], "core_block_source/v1");
	assert!(blocks["items"][0]["audit_history"].as_array().expect("audit history").len() >= 2);
	assert!(!blocks.to_string().contains("archival note must not appear"));

	let b_private = helpers::get_core_blocks(&app, TEST_AGENT_B, "private_only").await;

	assert_eq!(b_private["items"].as_array().expect("items array").len(), 0);

	let shared_block_id = helpers::create_core_block(
		&admin_app,
		"project_shared",
		"shared_operating_context",
		"Constraint: Shared core context requires explicit project grant and attachment.",
	)
	.await;
	let (denied_status, _) = helpers::attach_core_block(
		&admin_app,
		shared_block_id,
		TEST_AGENT_B,
		"private_plus_project",
	)
	.await;

	assert_eq!(denied_status, StatusCode::FORBIDDEN);

	helpers::insert_project_scope_grant(&state, TEST_AGENT_A, TEST_AGENT_A).await;

	let (shared_status, _) = helpers::attach_core_block(
		&admin_app,
		shared_block_id,
		TEST_AGENT_B,
		"private_plus_project",
	)
	.await;
	let b_shared = helpers::get_core_blocks(&app, TEST_AGENT_B, "private_plus_project").await;
	let b_wrong_profile = helpers::get_core_blocks(&app, TEST_AGENT_B, "private_only").await;

	assert_eq!(shared_status, StatusCode::OK);
	assert_eq!(b_shared["items"].as_array().expect("items array").len(), 1);
	assert_eq!(b_shared["items"][0]["scope"], "project_shared");
	assert_eq!(b_wrong_profile["items"].as_array().expect("items array").len(), 0);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
