use axum::{Router, body, http::StatusCode};
use serde_json::Value;
use uuid::Uuid;

use crate::helpers::{self, TEST_PROJECT_ID, TEST_TENANT_ID};
use elf_api::{routes, state::AppState};
use elf_config::{SecurityAuthKey, SecurityAuthRole};
use elf_testkit::TestDatabase;

async fn static_keys_admin_required_for_org_shared_writes_fixture()
-> Option<(TestDatabase, Router, Uuid)> {
	let (test_db, qdrant_url, collection) = helpers::test_env().await?;
	let mut config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "static_keys".to_string();
	config.security.auth_keys = vec![
		SecurityAuthKey {
			token_id: "user-token-id".to_string(),
			token: "user-token".to_string(),
			tenant_id: TEST_TENANT_ID.to_string(),
			project_id: TEST_PROJECT_ID.to_string(),
			agent_id: Some("user-agent".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::User,
		},
		SecurityAuthKey {
			token_id: "admin-token-id".to_string(),
			token: "admin-token".to_string(),
			tenant_id: TEST_TENANT_ID.to_string(),
			project_id: TEST_PROJECT_ID.to_string(),
			agent_id: Some("admin-agent".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::Admin,
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
		"Fact: org-shared publish setup note.",
	)
	.await;

	Some((test_db, app, note_id))
}

async fn static_keys_admin_required_for_org_shared_writes_requests(app: &Router, note_id: Uuid) {
	static_keys_admin_required_for_org_shared_writes_ingest_checks(app).await;
	static_keys_admin_required_for_org_shared_writes_publish_checks(app, note_id).await;
	static_keys_admin_required_for_org_shared_writes_grant_checks(app).await;
}

async fn static_keys_admin_required_for_org_shared_writes_ingest_checks(app: &Router) {
	let notes_payload = serde_json::json!({
		"scope": "org_shared",
		"notes": [{
			"type": "fact",
			"key": null,
			"text": "你好",
			"importance": 0.5,
			"confidence": 0.9,
			"ttl_days": null,
			"source_ref": {}
		}]
	})
	.to_string();
	let user_ingest = helpers::post_with_authorization_and_json_body(
		app,
		"/v2/notes/ingest",
		"Bearer user-token",
		&notes_payload,
		"Failed to build notes ingest request.",
		"Failed to call notes ingest.",
	)
	.await;

	assert_eq!(user_ingest.status(), StatusCode::FORBIDDEN);

	let admin_ingest = helpers::post_with_authorization_and_json_body(
		app,
		"/v2/notes/ingest",
		"Bearer admin-token",
		&notes_payload,
		"Failed to build notes ingest request.",
		"Failed to call notes ingest (admin).",
	)
	.await;

	assert_eq!(admin_ingest.status(), StatusCode::UNPROCESSABLE_ENTITY);

	let admin_ingest_body = body::to_bytes(admin_ingest.into_body(), usize::MAX)
		.await
		.expect("Failed to read notes ingest response body.");
	let admin_ingest_json: Value =
		serde_json::from_slice(&admin_ingest_body).expect("Failed to parse response.");

	assert_eq!(admin_ingest_json["error_code"], "NON_ENGLISH_INPUT");
}

async fn static_keys_admin_required_for_org_shared_writes_publish_checks(
	app: &Router,
	note_id: Uuid,
) {
	let publish_payload = serde_json::json!({ "space": "org_shared" }).to_string();
	let user_publish = helpers::post_with_authorization_and_json_body(
		app,
		&format!("/v2/notes/{note_id}/publish"),
		"Bearer user-token",
		&publish_payload,
		"Failed to build note publish request.",
		"Failed to call notes publish.",
	)
	.await;

	assert_eq!(user_publish.status(), StatusCode::FORBIDDEN);

	let admin_publish = helpers::post_with_authorization_and_json_body(
		app,
		&format!("/v2/notes/{note_id}/publish"),
		"Bearer admin-token",
		&publish_payload,
		"Failed to build note publish request.",
		"Failed to call notes publish (admin).",
	)
	.await;

	assert_eq!(admin_publish.status(), StatusCode::OK);
}

async fn static_keys_admin_required_for_org_shared_writes_grant_checks(app: &Router) {
	let grant_upsert_payload = serde_json::json!({ "grantee_kind": "project" }).to_string();
	let user_grant_upsert = helpers::post_with_authorization_and_json_body(
		app,
		"/v2/spaces/org_shared/grants",
		"Bearer user-token",
		&grant_upsert_payload,
		"Failed to build grant upsert request.",
		"Failed to call grant upsert.",
	)
	.await;

	assert_eq!(user_grant_upsert.status(), StatusCode::FORBIDDEN);

	let admin_grant_upsert = helpers::post_with_authorization_and_json_body(
		app,
		"/v2/spaces/org_shared/grants",
		"Bearer admin-token",
		&grant_upsert_payload,
		"Failed to build grant upsert request.",
		"Failed to call grant upsert (admin).",
	)
	.await;

	assert_eq!(admin_grant_upsert.status(), StatusCode::OK);

	let grant_revoke_payload = serde_json::json!({ "grantee_kind": "project" }).to_string();
	let user_grant_revoke = helpers::post_with_authorization_and_json_body(
		app,
		"/v2/spaces/org_shared/grants/revoke",
		"Bearer user-token",
		&grant_revoke_payload,
		"Failed to build grant revoke request.",
		"Failed to call grant revoke.",
	)
	.await;

	assert_eq!(user_grant_revoke.status(), StatusCode::FORBIDDEN);

	let admin_grant_revoke = helpers::post_with_authorization_and_json_body(
		app,
		"/v2/spaces/org_shared/grants/revoke",
		"Bearer admin-token",
		&grant_revoke_payload,
		"Failed to build grant revoke request.",
		"Failed to call grant revoke (admin).",
	)
	.await;

	assert_eq!(admin_grant_revoke.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn static_keys_admin_required_for_org_shared_writes() {
	let Some((test_db, app, note_id)) =
		static_keys_admin_required_for_org_shared_writes_fixture().await
	else {
		return;
	};

	static_keys_admin_required_for_org_shared_writes_requests(&app, note_id).await;

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
