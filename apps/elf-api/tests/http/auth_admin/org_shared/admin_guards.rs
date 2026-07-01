use axum::{
	body::Body,
	http::{Request, StatusCode},
};
use tower::util::ServiceExt as _;
use uuid::Uuid;

use crate::helpers;
use elf_api::{routes, state::AppState};
use elf_config::{SecurityAuthKey, SecurityAuthRole};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn static_keys_org_shared_ingest_requires_admin() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else { return };
	let mut config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "static_keys".to_string();
	config.security.auth_keys = vec![
		SecurityAuthKey {
			token_id: "user".to_string(),
			token: "user-token".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::User,
		},
		SecurityAuthKey {
			token_id: "admin".to_string(),
			token: "admin-token".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::Admin,
		},
	];

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let payload = serde_json::json!({
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
	});
	let response_user = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/notes/ingest")
				.header("Authorization", "Bearer user-token")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call notes ingest (user).");

	assert_eq!(response_user.status(), StatusCode::FORBIDDEN);

	let response_admin = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/notes/ingest")
				.header("Authorization", "Bearer admin-token")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call notes ingest (admin).");

	assert_eq!(response_admin.status(), StatusCode::UNPROCESSABLE_ENTITY);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn static_keys_org_shared_events_ingest_requires_admin() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else { return };
	let mut config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "static_keys".to_string();
	config.security.auth_keys = vec![
		SecurityAuthKey {
			token_id: "user".to_string(),
			token: "user-token".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::User,
		},
		SecurityAuthKey {
			token_id: "admin".to_string(),
			token: "admin-token".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::Admin,
		},
	];

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let payload = serde_json::json!({
		"scope": "org_shared",
		"dry_run": true,
		"messages": [{
			"role": "user",
			"content": "こんにちは"
		}]
	});
	let response_user = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/events/ingest")
				.header("Authorization", "Bearer user-token")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call events ingest (user).");

	assert_eq!(response_user.status(), StatusCode::FORBIDDEN);

	let response_admin = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/events/ingest")
				.header("Authorization", "Bearer admin-token")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call events ingest (admin).");

	assert_eq!(response_admin.status(), StatusCode::UNPROCESSABLE_ENTITY);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn static_keys_org_shared_publish_requires_admin() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else { return };
	let mut config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "static_keys".to_string();
	config.security.auth_keys = vec![
		SecurityAuthKey {
			token_id: "user".to_string(),
			token: "user-token".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::User,
		},
		SecurityAuthKey {
			token_id: "admin".to_string(),
			token: "admin-token".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::Admin,
		},
	];

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let note_id = Uuid::new_v4();
	let payload = serde_json::json!({"space":"org_shared"}).to_string();
	let response_user = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri(format!("/v2/notes/{note_id}/publish"))
				.header("Authorization", "Bearer user-token")
				.header("content-type", "application/json")
				.body(Body::from(payload.clone()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call note publish (user).");

	assert_eq!(response_user.status(), StatusCode::FORBIDDEN);

	let response_admin = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri(format!("/v2/notes/{note_id}/publish"))
				.header("Authorization", "Bearer admin-token")
				.header("content-type", "application/json")
				.body(Body::from(payload))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call note publish (admin).");

	assert_ne!(response_admin.status(), StatusCode::FORBIDDEN);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn static_keys_org_shared_grants_require_admin() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else { return };
	let mut config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "static_keys".to_string();
	config.security.auth_keys = vec![
		SecurityAuthKey {
			token_id: "user".to_string(),
			token: "user-token".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::User,
		},
		SecurityAuthKey {
			token_id: "admin".to_string(),
			token: "admin-token".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::Admin,
		},
	];

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let payload = serde_json::json!({"grantee_kind":"project","grantee_agent_id":null}).to_string();
	let response_user = app
		.clone()
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/spaces/org_shared/grants")
				.header("Authorization", "Bearer user-token")
				.header("content-type", "application/json")
				.body(Body::from(payload.clone()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call grant upsert (user).");

	assert_eq!(response_user.status(), StatusCode::FORBIDDEN);

	let response_admin = app
		.oneshot(
			Request::builder()
				.method("POST")
				.uri("/v2/spaces/org_shared/grants")
				.header("Authorization", "Bearer admin-token")
				.header("content-type", "application/json")
				.body(Body::from(payload))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call grant upsert (admin).");

	assert_ne!(response_admin.status(), StatusCode::FORBIDDEN);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
