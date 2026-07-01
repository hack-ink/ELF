use axum::http::HeaderMap;

use crate::routes::{
	self, HEADER_AGENT_ID, HEADER_AUTHORIZATION, HEADER_PROJECT_ID, HEADER_READ_PROFILE,
	HEADER_TENANT_ID, HEADER_TRUSTED_TOKEN_ID,
};
use elf_config::{SecurityAuthKey, SecurityAuthRole};

#[test]
fn apply_auth_key_context_overrides_headers() {
	let mut headers = HeaderMap::new();

	headers.insert(HEADER_AUTHORIZATION, "Bearer old".parse().expect("invalid header"));
	headers.insert(HEADER_TENANT_ID, "bad-tenant".parse().expect("invalid header"));
	headers.insert(HEADER_PROJECT_ID, "bad-project".parse().expect("invalid header"));
	headers.insert(HEADER_AGENT_ID, "bad-agent".parse().expect("invalid header"));
	headers.insert(HEADER_READ_PROFILE, "private_only".parse().expect("invalid header"));
	headers.insert(HEADER_TRUSTED_TOKEN_ID, "old-id".parse().expect("invalid header"));

	let key = SecurityAuthKey {
		token_id: "k1".to_string(),
		token: "secret".to_string(),
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: Some("a".to_string()),
		read_profile: "all_scopes".to_string(),
		role: SecurityAuthRole::Admin,
	};

	routes::apply_auth_key_context(&mut headers, &key).expect("Expected context injection.");

	assert_eq!(
		headers.get(HEADER_TENANT_ID).and_then(|v| v.to_str().ok()).expect("missing tenant"),
		"t"
	);
	assert_eq!(
		headers.get(HEADER_PROJECT_ID).and_then(|v| v.to_str().ok()).expect("missing project"),
		"p"
	);
	assert_eq!(
		headers.get(HEADER_AGENT_ID).and_then(|v| v.to_str().ok()).expect("missing agent"),
		"a"
	);
	assert_eq!(
		headers
			.get(HEADER_READ_PROFILE)
			.and_then(|v| v.to_str().ok())
			.expect("missing read profile"),
		"all_scopes"
	);
	assert_eq!(
		headers
			.get(HEADER_TRUSTED_TOKEN_ID)
			.and_then(|v| v.to_str().ok())
			.expect("missing trusted token_id"),
		"k1"
	);
}

#[test]
fn apply_auth_key_context_requires_agent_scope() {
	let mut headers = HeaderMap::new();
	let key = SecurityAuthKey {
		token_id: "k1".to_string(),
		token: "secret".to_string(),
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: None,
		read_profile: "all_scopes".to_string(),
		role: SecurityAuthRole::User,
	};
	let err = routes::apply_auth_key_context(&mut headers, &key)
		.expect_err("Expected forbidden error for missing agent_id.");

	assert_eq!(err.status, axum::http::StatusCode::FORBIDDEN);
}

#[test]
fn effective_token_id_ignores_header_when_auth_mode_off() {
	let mut headers = HeaderMap::new();

	headers.insert(HEADER_TRUSTED_TOKEN_ID, "user-supplied".parse().expect("invalid header"));

	assert_eq!(routes::effective_token_id("off", &headers), None);
}

#[test]
fn effective_token_id_uses_header_when_auth_mode_static_keys() {
	let mut headers = HeaderMap::new();

	headers.insert(HEADER_TRUSTED_TOKEN_ID, "k1".parse().expect("invalid header"));

	assert_eq!(routes::effective_token_id("static_keys", &headers), Some("k1".to_string()));
}

#[test]
fn sanitize_trusted_token_header_removes_header() {
	let mut headers = HeaderMap::new();

	headers.insert(HEADER_TRUSTED_TOKEN_ID, "user-supplied".parse().expect("invalid header"));

	routes::sanitize_trusted_token_header(&mut headers);

	assert!(headers.get(HEADER_TRUSTED_TOKEN_ID).is_none());
}
