use axum::{
	Router,
	body::{self, Body},
	http::{Request, StatusCode},
};
use serde_json::Value;
use tower::util::ServiceExt as _;
use uuid::Uuid;

use crate::helpers::{self, TEST_PROJECT_ID, TEST_PROJECT_ID_B, TEST_TENANT_ID};
use elf_api::{routes, state::AppState};
use elf_config::{SecurityAuthKey, SecurityAuthRole};
use elf_testkit::TestDatabase;

pub(crate) async fn org_shared_note_is_visible_across_projects_fixture()
-> Option<(TestDatabase, Router, AppState, Uuid)> {
	let (test_db, qdrant_url, collection) = helpers::test_env().await?;
	let mut config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "static_keys".to_string();
	config.security.auth_keys = vec![
		SecurityAuthKey {
			token_id: "admin-token-id".to_string(),
			token: "admin-token".to_string(),
			tenant_id: TEST_TENANT_ID.to_string(),
			project_id: TEST_PROJECT_ID.to_string(),
			agent_id: Some("admin-agent".to_string()),
			read_profile: "all_scopes".to_string(),
			role: SecurityAuthRole::Admin,
		},
		SecurityAuthKey {
			token_id: "reader-token-id".to_string(),
			token: "reader-token".to_string(),
			tenant_id: TEST_TENANT_ID.to_string(),
			project_id: TEST_PROJECT_ID_B.to_string(),
			agent_id: Some("reader-agent".to_string()),
			read_profile: "all_scopes".to_string(),
			role: SecurityAuthRole::User,
		},
	];

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state.clone());
	let note_id = Uuid::new_v4();

	helpers::insert_note(
		&state,
		note_id,
		"agent_private",
		"admin-agent",
		"Fact: org_shared cross-project visibility.",
	)
	.await;

	Some((test_db, app, state, note_id))
}

pub(crate) async fn list_org_shared_notes_as_reader(app: &Router) -> Value {
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method("GET")
				.uri("/v2/notes?scope=org_shared")
				.header("Authorization", "Bearer reader-token")
				.body(Body::empty())
				.expect("Failed to build list request."),
		)
		.await
		.expect("Failed to call notes list.");

	assert_eq!(response.status(), StatusCode::OK);

	let body = body::to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Failed to read list response body.");

	serde_json::from_slice(&body).expect("Failed to parse list response.")
}

pub(crate) async fn publish_org_shared_note_as_reader_can_see(scope_app: &Router, note_id: Uuid) {
	let payload = serde_json::json!({ "space": "org_shared" }).to_string();
	let response = scope_app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri(format!("/v2/notes/{note_id}/publish"))
				.header("Authorization", "Bearer admin-token")
				.header("content-type", "application/json")
				.body(Body::from(payload))
				.expect("Failed to build note publish request."),
		)
		.await
		.expect("Failed to call notes publish.");

	assert_eq!(response.status(), StatusCode::OK);
}

pub(crate) async fn assert_note_visible_to_project_reader(
	scope_app: &Router,
	state: &AppState,
	note_id: Uuid,
) {
	let (scope, project_id) = helpers::note_scope_and_project_id(state, note_id).await;

	assert_eq!(scope, "org_shared");
	// org_shared note rows live in the synthetic org project, not the request project.
	assert_eq!(project_id, "__org__");

	let org_grant_count =
		helpers::active_org_shared_project_grant_count(state, "admin-agent").await;

	assert!(org_grant_count > 0);

	// org_shared grant rows live in '__org__' as well; they should not be written into the request
	// project.
	let request_project_grant_count = helpers::active_org_shared_project_grant_count_for_project(
		state,
		TEST_PROJECT_ID,
		"admin-agent",
	)
	.await;

	assert_eq!(request_project_grant_count, 0);

	let list_after_json = list_org_shared_notes_as_reader(scope_app).await;
	let items = list_after_json["items"].as_array().expect("Missing items array.");
	let ids: Vec<&str> = items.iter().filter_map(|item| item["note_id"].as_str()).collect();
	let note_id_str = note_id.to_string();

	assert!(ids.contains(&note_id_str.as_str()));
}
