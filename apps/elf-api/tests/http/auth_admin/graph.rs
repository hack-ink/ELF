use axum::{
	body::{self, Body},
	http::{Request, StatusCode},
};
use serde_json::Value;
use tower::util::ServiceExt as _;
use uuid::Uuid;

use crate::helpers;
use elf_api::{routes, state::AppState};
use elf_config::{SecurityAuthKey, SecurityAuthRole};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn global_graph_predicate_write_requires_super_admin() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else {
		return;
	};
	let mut config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);

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
	let json: Value = serde_json::from_slice(&body).expect("Failed to parse response.");

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
