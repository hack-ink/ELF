use axum::{
	body::{self, Body},
	http::{Request, StatusCode},
};
use serde_json::Value;
use tower::util::ServiceExt as _;
use uuid::Uuid;

use crate::helpers::{self, TEST_AGENT_A, TEST_PROJECT_ID, TEST_TENANT_ID};
use elf_api::{routes, state::AppState};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn admin_note_provenance_includes_request_id_on_success() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else {
		return;
	};
	let mut config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "off".to_string();

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::admin_router(state.clone());
	let note_id = Uuid::new_v4();
	let request_id = Uuid::new_v4();

	helpers::insert_note(
		&state,
		note_id,
		"agent_private",
		TEST_AGENT_A,
		"Provenance integration test note.",
	)
	.await;

	let response = app
		.oneshot(
			Request::builder()
				.uri(format!("/v2/admin/notes/{note_id}/provenance"))
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_A)
				.header("X-ELF-Request-Id", request_id.to_string())
				.body(Body::empty())
				.expect("Failed to build provenance request."),
		)
		.await
		.expect("Failed to call admin note provenance.");

	assert_eq!(response.status(), StatusCode::OK);

	let expected_request_id = request_id.to_string();

	assert_eq!(
		response.headers().get("X-ELF-Request-Id").and_then(|value| value.to_str().ok()),
		Some(expected_request_id.as_str())
	);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read provenance response body.");
	let json: Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["schema"], "elf.note_provenance_bundle/v1");
	assert_eq!(json["request_id"], request_id.to_string());

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn admin_note_history_includes_request_id_on_success() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else {
		return;
	};
	let mut config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "off".to_string();

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::admin_router(state.clone());
	let note_id = Uuid::new_v4();
	let request_id = Uuid::new_v4();

	helpers::insert_note(
		&state,
		note_id,
		"agent_private",
		TEST_AGENT_A,
		"History integration test note.",
	)
	.await;

	let response = app
		.oneshot(
			Request::builder()
				.uri(format!("/v2/admin/notes/{note_id}/history"))
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_A)
				.header("X-ELF-Request-Id", request_id.to_string())
				.body(Body::empty())
				.expect("Failed to build history request."),
		)
		.await
		.expect("Failed to call admin note history.");

	assert_eq!(response.status(), StatusCode::OK);

	let expected_request_id = request_id.to_string();

	assert_eq!(
		response.headers().get("X-ELF-Request-Id").and_then(|value| value.to_str().ok()),
		Some(expected_request_id.as_str())
	);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read history response body.");
	let json: Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["schema"], "elf.memory_history/v1");
	assert_eq!(json["request_id"], request_id.to_string());

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn admin_note_provenance_rejects_invalid_request_id_header() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else {
		return;
	};
	let mut config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "off".to_string();

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::admin_router(state);
	let note_id = Uuid::new_v4();
	let response = app
		.oneshot(
			Request::builder()
				.uri(format!("/v2/admin/notes/{note_id}/provenance"))
				.header("X-ELF-Request-Id", "not-a-uuid")
				.body(Body::empty())
				.expect("Failed to build provenance request."),
		)
		.await
		.expect("Failed to call admin note provenance.");
	let response_request_id = response
		.headers()
		.get("X-ELF-Request-Id")
		.and_then(|value| value.to_str().ok())
		.expect("Expected request id header in error response.");
	let generated_request_id = Uuid::parse_str(response_request_id)
		.expect("Expected valid generated request_id in response header.");

	assert_eq!(response.status(), StatusCode::BAD_REQUEST);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read provenance response body.");
	let json: Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["error_code"], "INVALID_REQUEST");
	assert_eq!(json["fields"][0], "$.headers.X-ELF-Request-Id");
	assert_eq!(json["request_id"], serde_json::Value::String(generated_request_id.to_string()),);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
