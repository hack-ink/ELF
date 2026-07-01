use axum::{
	body::Body,
	http::{Request, StatusCode},
};
use tower::util::ServiceExt as _;

use crate::helpers;
use elf_api::{routes, state::AppState};
use elf_config::{SecurityAuthKey, SecurityAuthRole};

#[tokio::test]
#[ignore = "Requires external Postgres and Qdrant. Set ELF_PG_DSN and ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to run."]
async fn static_keys_requires_bearer_header() {
	let Some((test_db, qdrant_url, collection)) = helpers::test_env().await else {
		return;
	};
	let mut config = helpers::test_config(test_db.dsn().to_string(), qdrant_url, collection);

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
