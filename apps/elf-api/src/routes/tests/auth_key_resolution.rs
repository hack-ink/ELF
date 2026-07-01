use axum::http::HeaderMap;

use crate::routes::{self, HEADER_AUTHORIZATION};
use elf_config::{SecurityAuthKey, SecurityAuthRole};

#[test]
fn resolve_auth_key_requires_bearer_header() {
	let headers = HeaderMap::new();
	let keys = vec![SecurityAuthKey {
		token_id: "k1".to_string(),
		token: "secret".to_string(),
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: Some("a".to_string()),
		read_profile: "private_plus_project".to_string(),
		role: SecurityAuthRole::User,
	}];
	let err = routes::resolve_auth_key(&headers, &keys).expect_err("Expected unauthorized error.");

	assert_eq!(err.status, axum::http::StatusCode::UNAUTHORIZED);
}

#[test]
fn resolve_auth_key_rejects_unknown_token() {
	let keys = vec![SecurityAuthKey {
		token_id: "k1".to_string(),
		token: "secret".to_string(),
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: Some("a".to_string()),
		read_profile: "private_plus_project".to_string(),
		role: SecurityAuthRole::User,
	}];
	let mut headers = HeaderMap::new();

	headers.insert(HEADER_AUTHORIZATION, "Bearer wrong".parse().expect("invalid header"));

	let err = routes::resolve_auth_key(&headers, &keys)
		.expect_err("Expected unauthorized error for bad key.");

	assert_eq!(err.status, axum::http::StatusCode::UNAUTHORIZED);
}

#[test]
fn resolve_auth_key_rejects_non_bearer_authorization() {
	let keys = vec![SecurityAuthKey {
		token_id: "k1".to_string(),
		token: "secret".to_string(),
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: Some("a".to_string()),
		read_profile: "private_plus_project".to_string(),
		role: SecurityAuthRole::User,
	}];
	let mut headers = HeaderMap::new();

	headers.insert(HEADER_AUTHORIZATION, "Token secret".parse().expect("invalid header"));

	let err = routes::resolve_auth_key(&headers, &keys)
		.expect_err("Expected unauthorized error for non-bearer authorization.");

	assert_eq!(err.status, axum::http::StatusCode::UNAUTHORIZED);
}

#[test]
fn resolve_auth_key_rejects_lowercase_bearer_prefix() {
	let keys = vec![SecurityAuthKey {
		token_id: "k1".to_string(),
		token: "secret".to_string(),
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: Some("a".to_string()),
		read_profile: "private_plus_project".to_string(),
		role: SecurityAuthRole::User,
	}];
	let mut headers = HeaderMap::new();

	headers.insert(HEADER_AUTHORIZATION, "bearer secret".parse().expect("invalid header"));

	let err = routes::resolve_auth_key(&headers, &keys)
		.expect_err("Expected unauthorized error for lowercase bearer prefix.");

	assert_eq!(err.status, axum::http::StatusCode::UNAUTHORIZED);
}
