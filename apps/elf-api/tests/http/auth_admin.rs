use super::*;

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn static_keys_requires_bearer_header() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let mut config = test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "static_keys".to_string();
	config.security.auth_keys = vec![SecurityAuthKey {
		token_id: "k1".to_string(),
		token: "secret".to_string(),
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: Some("a".to_string()),
		read_profile: "private_plus_project".to_string(),
		role: SecurityAuthRole::User,
	}];

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::router(state);
	let no_auth = app
		.clone()
		.oneshot(Request::builder().uri("/health").body(Body::empty()).expect("build request"))
		.await
		.expect("call /health without auth");

	assert_eq!(no_auth.status(), StatusCode::UNAUTHORIZED);

	let non_bearer_auth = app
		.clone()
		.oneshot(
			Request::builder()
				.uri("/health")
				.header("Authorization", "Basic secret")
				.body(Body::empty())
				.expect("build non-bearer auth request"),
		)
		.await
		.expect("call /health with non-bearer auth");

	assert_eq!(non_bearer_auth.status(), StatusCode::UNAUTHORIZED);

	let bearer_auth = app
		.oneshot(
			Request::builder()
				.uri("/health")
				.header("Authorization", "Bearer secret")
				.body(Body::empty())
				.expect("build bearer auth request"),
		)
		.await
		.expect("call /health with bearer auth");

	assert_eq!(bearer_auth.status(), StatusCode::OK);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

async fn static_keys_admin_required_for_org_shared_writes_fixture()
-> Option<(TestDatabase, Router, Uuid)> {
	let (test_db, qdrant_url, collection) = test_env().await?;
	let mut config = test_config(test_db.dsn().to_string(), qdrant_url, collection);

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

	insert_note(
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
	let user_ingest = post_with_authorization_and_json_body(
		app,
		"/v2/notes/ingest",
		"Bearer user-token",
		&notes_payload,
		"Failed to build notes ingest request.",
		"Failed to call notes ingest.",
	)
	.await;

	assert_eq!(user_ingest.status(), StatusCode::FORBIDDEN);

	let admin_ingest = post_with_authorization_and_json_body(
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
	let admin_ingest_json: serde_json::Value =
		serde_json::from_slice(&admin_ingest_body).expect("Failed to parse response.");

	assert_eq!(admin_ingest_json["error_code"], "NON_ENGLISH_INPUT");
}

async fn static_keys_admin_required_for_org_shared_writes_publish_checks(
	app: &Router,
	note_id: Uuid,
) {
	let publish_payload = serde_json::json!({ "space": "org_shared" }).to_string();
	let user_publish = post_with_authorization_and_json_body(
		app,
		&format!("/v2/notes/{note_id}/publish"),
		"Bearer user-token",
		&publish_payload,
		"Failed to build note publish request.",
		"Failed to call notes publish.",
	)
	.await;

	assert_eq!(user_publish.status(), StatusCode::FORBIDDEN);

	let admin_publish = post_with_authorization_and_json_body(
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
	let user_grant_upsert = post_with_authorization_and_json_body(
		app,
		"/v2/spaces/org_shared/grants",
		"Bearer user-token",
		&grant_upsert_payload,
		"Failed to build grant upsert request.",
		"Failed to call grant upsert.",
	)
	.await;

	assert_eq!(user_grant_upsert.status(), StatusCode::FORBIDDEN);

	let admin_grant_upsert = post_with_authorization_and_json_body(
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
	let user_grant_revoke = post_with_authorization_and_json_body(
		app,
		"/v2/spaces/org_shared/grants/revoke",
		"Bearer user-token",
		&grant_revoke_payload,
		"Failed to build grant revoke request.",
		"Failed to call grant revoke.",
	)
	.await;

	assert_eq!(user_grant_revoke.status(), StatusCode::FORBIDDEN);

	let admin_grant_revoke = post_with_authorization_and_json_body(
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

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn static_keys_org_shared_ingest_requires_admin() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else { return };
	let mut config = test_config(test_db.dsn().to_string(), qdrant_url, collection);

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
	let Some((test_db, qdrant_url, collection)) = test_env().await else { return };
	let mut config = test_config(test_db.dsn().to_string(), qdrant_url, collection);

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
	let Some((test_db, qdrant_url, collection)) = test_env().await else { return };
	let mut config = test_config(test_db.dsn().to_string(), qdrant_url, collection);

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
	let Some((test_db, qdrant_url, collection)) = test_env().await else { return };
	let mut config = test_config(test_db.dsn().to_string(), qdrant_url, collection);

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

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn admin_note_provenance_includes_request_id_on_success() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let mut config = test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "off".to_string();

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::admin_router(state.clone());
	let note_id = Uuid::new_v4();
	let request_id = Uuid::new_v4();

	insert_note(
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
	let json: serde_json::Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["schema"], "elf.note_provenance_bundle/v1");
	assert_eq!(json["request_id"], request_id.to_string());

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn admin_note_history_includes_request_id_on_success() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let mut config = test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "off".to_string();

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::admin_router(state.clone());
	let note_id = Uuid::new_v4();
	let request_id = Uuid::new_v4();

	insert_note(&state, note_id, "agent_private", TEST_AGENT_A, "History integration test note.")
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
	let json: serde_json::Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["schema"], "elf.memory_history/v1");
	assert_eq!(json["request_id"], request_id.to_string());

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn admin_note_provenance_rejects_invalid_request_id_header() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let mut config = test_config(test_db.dsn().to_string(), qdrant_url, collection);

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
	let json: serde_json::Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["error_code"], "INVALID_REQUEST");
	assert_eq!(json["fields"][0], "$.headers.X-ELF-Request-Id");
	assert_eq!(json["request_id"], serde_json::Value::String(generated_request_id.to_string()),);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn global_graph_predicate_write_requires_super_admin() {
	let Some((test_db, qdrant_url, collection)) = test_env().await else {
		return;
	};
	let mut config = test_config(test_db.dsn().to_string(), qdrant_url, collection);

	config.security.auth_mode = "static_keys".to_string();
	config.security.auth_keys = vec![
		SecurityAuthKey {
			token_id: "admin".to_string(),
			token: "admin-token".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::Admin,
		},
		SecurityAuthKey {
			token_id: "super".to_string(),
			token: "super-token".to_string(),
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: Some("a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::SuperAdmin,
		},
	];

	let state = AppState::new(config).await.expect("Failed to initialize app state.");
	let app = routes::admin_router(state.clone());
	let predicate_id = Uuid::new_v4();

	sqlx::query(
		"\
	INSERT INTO graph_predicates (
		predicate_id,
		scope_key,
		tenant_id,
		project_id,
		canonical,
		canonical_norm,
		cardinality,
		status,
		created_at,
		updated_at
	)
	VALUES ($1, '__global__', NULL, NULL, 'global_test', 'global_test', 'multi', 'pending', now(), now())",
	)
	.bind(predicate_id)
	.execute(&state.service.db.pool)
	.await
	.expect("Failed to insert global predicate.");

	let payload = serde_json::json!({ "status": "active" });
	let response_admin = app
		.clone()
		.oneshot(
			Request::builder()
				.method("PATCH")
				.uri(format!("/v2/admin/graph/predicates/{predicate_id}"))
				.header("Authorization", "Bearer admin-token")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call admin graph predicate patch (admin).");

	assert_eq!(response_admin.status(), StatusCode::FORBIDDEN);

	let body = body::to_bytes(response_admin.into_body(), usize::MAX)
		.await
		.expect("Failed to read response body.");
	let json: serde_json::Value = serde_json::from_slice(&body).expect("Failed to parse response.");

	assert_eq!(json["error_code"], "SCOPE_DENIED");

	let response_super = app
		.oneshot(
			Request::builder()
				.method("PATCH")
				.uri(format!("/v2/admin/graph/predicates/{predicate_id}"))
				.header("Authorization", "Bearer super-token")
				.header("content-type", "application/json")
				.body(Body::from(payload.to_string()))
				.expect("Failed to build request."),
		)
		.await
		.expect("Failed to call admin graph predicate patch (super_admin).");

	assert_eq!(response_super.status(), StatusCode::OK);

	test_db.cleanup().await.expect("Failed to cleanup test database.");
}
