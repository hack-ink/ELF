use axum::{
	body::{self, Body},
	http::{Request, StatusCode},
};
use serde_json::Value;
use tower::util::ServiceExt as _;
use uuid::Uuid;

use crate::helpers::{self, TEST_AGENT_A, TEST_AGENT_B, TEST_PROJECT_ID, TEST_TENANT_ID};
use elf_api::{routes, state::AppState};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn sharing_visibility_requires_explicit_project_grant() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else {
		return;
	};
	let config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state.clone());
	let note_id = Uuid::new_v4();

	helpers::insert_note(
		&state,
		note_id,
		"project_shared",
		TEST_AGENT_A,
		"Fact: shared note without grant",
	)
	.await;

	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("GET")
				.uri("/v2/notes?scope=project_shared")
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_B)
				.body(Body::empty())
				.expect("Failed to build list request."),
		)
		.await
		.expect("Failed to call notes list.");

	assert_eq!(response.status(), StatusCode::OK);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read list response body.");
	let list_json: Value = serde_json::from_slice(&body).expect("Failed to parse list response.");

	assert_eq!(list_json["items"].as_array().expect("Missing items array.").len(), 0);

	let note_response = app
		.clone()
		.oneshot(
			Request::builder()
				.uri(format!("/v2/notes/{note_id}"))
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_B)
				.body(Body::empty())
				.expect("Failed to build get request."),
		)
		.await
		.expect("Failed to call notes get.");

	assert_eq!(note_response.status(), StatusCode::BAD_REQUEST);

	let body = body::to_bytes(note_response.into_body(), usize::MAX)
		.await
		.expect("Failed to read get response body.");
	let note_json: Value = serde_json::from_slice(&body).expect("Failed to parse get response.");

	assert_eq!(note_json["error_code"], "INVALID_REQUEST");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn sharing_project_grant_enables_agent_access_to_shared_note() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else {
		return;
	};
	let config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);
	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state.clone());
	let note_id = Uuid::new_v4();

	helpers::insert_note(
		&state,
		note_id,
		"project_shared",
		TEST_AGENT_A,
		"Fact: shared note with explicit grant.",
	)
	.await;
	helpers::insert_project_scope_grant(&state, TEST_AGENT_A, TEST_AGENT_A).await;

	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("GET")
				.uri("/v2/notes?scope=project_shared")
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_B)
				.body(Body::empty())
				.expect("Failed to build list request."),
		)
		.await
		.expect("Failed to call notes list.");

	assert_eq!(response.status(), StatusCode::OK);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read list response body.");
	let list_json: Value = serde_json::from_slice(&body).expect("Failed to parse list response.");
	let items = list_json["items"].as_array().expect("Missing items array.");

	assert_eq!(items.len(), 1);
	assert_eq!(items[0]["note_id"], note_id.to_string());

	let note_response = app
		.clone()
		.oneshot(
			Request::builder()
				.uri(format!("/v2/notes/{note_id}"))
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_B)
				.body(Body::empty())
				.expect("Failed to build get request."),
		)
		.await
		.expect("Failed to call notes get.");

	assert_eq!(note_response.status(), StatusCode::OK);

	let body = body::to_bytes(note_response.into_body(), usize::MAX)
		.await
		.expect("Failed to read get response body.");
	let note_json: Value = serde_json::from_slice(&body).expect("Failed to parse get response.");

	assert_eq!(note_json["note_id"], note_id.to_string());
	assert_eq!(note_json["scope"], "project_shared");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
