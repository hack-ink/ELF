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
async fn sharing_publish_creates_scope_and_grant_visibility() {
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
		"agent_private",
		TEST_AGENT_A,
		"Fact: private note for publish test.",
	)
	.await;

	let initial_grant_count = helpers::active_project_grant_count(&state, TEST_AGENT_A).await;

	assert_eq!(initial_grant_count, 0);

	let publish_payload = serde_json::json!({"space":"team_shared"}).to_string();
	let publish_response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri(format!("/v2/notes/{note_id}/publish"))
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_A)
				.header("content-type", "application/json")
				.body(Body::from(publish_payload))
				.expect("Failed to build publish request."),
		)
		.await
		.expect("Failed to call note publish.");

	assert_eq!(publish_response.status(), StatusCode::OK);

	let publish_body = body::to_bytes(publish_response.into_body(), usize::MAX)
		.await
		.expect("Failed to read publish response body.");
	let publish_json: Value =
		serde_json::from_slice(&publish_body).expect("Failed to parse publish response.");

	assert_eq!(publish_json["note_id"], note_id.to_string());
	assert_eq!(publish_json["space"], "team_shared");

	let after_grant_count = helpers::active_project_grant_count(&state, TEST_AGENT_A).await;

	assert_eq!(after_grant_count, 1);

	let list_response = app
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

	assert_eq!(list_response.status(), StatusCode::OK);

	let list_body = body::to_bytes(list_response.into_body(), usize::MAX)
		.await
		.expect("Failed to read list response body.");
	let list_json: Value =
		serde_json::from_slice(&list_body).expect("Failed to parse list response.");
	let items = list_json["items"].as_array().expect("Missing items array.");

	assert_eq!(items.len(), 1);
	assert_eq!(items[0]["note_id"], note_id.to_string());

	let get_response = app
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

	assert_eq!(get_response.status(), StatusCode::OK);

	let get_body = body::to_bytes(get_response.into_body(), usize::MAX)
		.await
		.expect("Failed to read get response body.");
	let get_json: Value = serde_json::from_slice(&get_body).expect("Failed to parse get response.");

	assert_eq!(get_json["note_id"], note_id.to_string());
	assert_eq!(get_json["scope"], "project_shared");

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn sharing_revoke_project_grant_removes_visibility() {
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
		"Fact: shared note for revoke test.",
	)
	.await;
	helpers::insert_project_scope_grant(&state, TEST_AGENT_A, TEST_AGENT_A).await;

	let grant_count_before = helpers::active_project_grant_count(&state, TEST_AGENT_A).await;

	assert_eq!(grant_count_before, 1);

	let list_before = app
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
	let list_before_body = body::to_bytes(list_before.into_body(), usize::MAX)
		.await
		.expect("Failed to read list response body.");
	let list_before_json: Value =
		serde_json::from_slice(&list_before_body).expect("Failed to parse list response.");

	assert_eq!(list_before_json["items"].as_array().expect("Missing items array.").len(), 1);

	let revoke_payload = serde_json::json!({"grantee_kind":"project"}).to_string();
	let revoke_response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/spaces/team_shared/grants/revoke")
				.header("X-ELF-Tenant-Id", TEST_TENANT_ID)
				.header("X-ELF-Project-Id", TEST_PROJECT_ID)
				.header("X-ELF-Agent-Id", TEST_AGENT_A)
				.header("content-type", "application/json")
				.body(Body::from(revoke_payload))
				.expect("Failed to build revoke request."),
		)
		.await
		.expect("Failed to call grant revoke.");

	assert_eq!(revoke_response.status(), StatusCode::OK);

	let grant_count_after = helpers::active_project_grant_count(&state, TEST_AGENT_A).await;

	assert_eq!(grant_count_after, 0);

	let list_after = app
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

	assert_eq!(list_after.status(), StatusCode::OK);

	let list_after_body = body::to_bytes(list_after.into_body(), usize::MAX)
		.await
		.expect("Failed to read list response body.");
	let list_after_json: Value =
		serde_json::from_slice(&list_after_body).expect("Failed to parse list response.");

	assert_eq!(list_after_json["items"].as_array().expect("Missing items array.").len(), 0);

	let get_after = app
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

	assert_eq!(get_after.status(), StatusCode::BAD_REQUEST);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
