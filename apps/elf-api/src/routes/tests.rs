use axum::http::HeaderMap;
use uuid::Uuid;

use crate::routes::{
	self, ADMIN_VIEWER_PATH, HEADER_AGENT_ID, HEADER_AUTHORIZATION, HEADER_PROJECT_ID,
	HEADER_READ_PROFILE, HEADER_REQUEST_ID, HEADER_TENANT_ID, HEADER_TRUSTED_TOKEN_ID,
};
use elf_config::{SecurityAuthKey, SecurityAuthRole};

#[test]
fn require_admin_for_org_shared_writes_denies_user_in_static_keys_mode() {
	let err =
		routes::require_admin_for_org_shared_writes("static_keys", Some(SecurityAuthRole::User))
			.expect_err("Expected forbidden error for non-admin role.");

	assert_eq!(err.status, axum::http::StatusCode::FORBIDDEN);
}

#[test]
fn require_admin_for_org_shared_writes_allows_admin_in_static_keys_mode() {
	routes::require_admin_for_org_shared_writes("static_keys", Some(SecurityAuthRole::Admin))
		.expect("Expected admin role to be allowed.");
}

#[test]
fn require_admin_for_org_shared_writes_allows_superadmin_in_static_keys_mode() {
	routes::require_admin_for_org_shared_writes("static_keys", Some(SecurityAuthRole::SuperAdmin))
		.expect("Expected superadmin role to be allowed.");
}

#[test]
fn require_admin_for_org_shared_writes_allows_non_static_keys_auth_mode() {
	routes::require_admin_for_org_shared_writes("off", None)
		.expect("Expected auth_mode != static_keys.");
}

#[test]
fn admin_viewer_uses_admin_operator_routes_without_raw_memory_bypasses() {
	let html = routes::VIEWER_HTML;

	assert_eq!(ADMIN_VIEWER_PATH, "/viewer");
	assert!(html.contains("/v2/admin/searches"));
	assert!(html.contains("/v2/admin/docs/search/l0"));
	assert!(html.contains("/v2/admin/docs/excerpts"));
	assert!(html.contains("/v2/admin/docs/${encodeURIComponent(item.doc_id)}"));
	assert!(html.contains("/v2/admin/dreaming/review-queue"));
	assert!(
		html.contains("/v2/admin/consolidation/proposals/${encodeURIComponent(proposalId)}/review")
	);
	assert!(html.contains("/v2/admin/notes/${encodeURIComponent(noteId)}/history"));
	assert!(html.contains("/v2/admin/notes/${encodeURIComponent(noteId)}/corrections"));
	assert!(html.contains("/v2/admin/recall-debug/panel"));
	assert!(html.contains("/v2/admin/traces/recent"));
	assert!(html.contains("/v2/admin/traces/${encodeURIComponent(traceId)}/bundle"));
	assert!(html.contains("/v2/admin/notes/"));
	assert!(html.contains("/v2/admin/knowledge/pages/search"));
	assert!(html.contains("mode: \"full\""));
	assert!(html.contains("candidates_limit: 200"));
	assert!(html.contains("Replay Candidates"));
	assert!(html.contains("Selected Final Results"));
	assert!(html.contains("Providers And Ranking"));
	assert!(html.contains("Relation Context"));
	assert!(html.contains("Knowledge Page Snippets"));
	assert!(html.contains("Derived page: source documents"));
	assert!(html.contains("Source Library"));
	assert!(html.contains("Memory Inbox"));
	assert!(html.contains("Memory History"));
	assert!(html.contains("Recall Debug"));
	assert!(html.contains("Apply Ledger Correction"));
	assert!(html.contains("Apply / Supersede"));
	assert!(html.contains("directTraceId"));
	assert!(html.contains("trace_id"));
	assert!(html.contains("loadInitialTrace"));
	assert!(!html.contains("method: \"PATCH\""));
	assert!(!html.contains("method: \"PUT\""));
	assert!(!html.contains("method: \"DELETE\""));
	assert!(!html.contains("/v2/notes/ingest"));
	assert!(!html.contains("/v2/events/ingest"));
	assert!(!html.contains("/publish"));
}

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

#[test]
fn parse_request_id_from_headers_generates_when_missing() {
	let headers = HeaderMap::new();
	let request_id = routes::parse_request_id_from_headers(&headers)
		.expect("Expected a generated request ID when header is missing.");

	assert_ne!(request_id.to_string(), Uuid::nil().to_string());
}

#[test]
fn parse_request_id_from_headers_rejects_invalid() {
	let mut headers = HeaderMap::new();

	headers.insert(HEADER_REQUEST_ID, "not-a-uuid".parse().expect("invalid request_id"));

	let err = routes::parse_request_id_from_headers(&headers)
		.expect_err("Expected invalid request_id to be rejected.");

	assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
	assert_eq!(err.error_code, "INVALID_REQUEST");
	assert_eq!(err.fields, Some(vec![format!("$.headers.{HEADER_REQUEST_ID}")]));
}

#[test]
fn inject_request_id_into_json_body_adds_request_id_to_object() {
	let request_id = Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").expect("valid uuid");
	let body = serde_json::json!({"note_id":"abc","status":"ok"}).to_string();
	let response_body = routes::inject_request_id_into_json_body(body.as_bytes(), &request_id)
		.expect("Expected request_id field to be injected.");
	let response_value =
		serde_json::from_slice::<serde_json::Value>(&response_body).expect("Expected valid JSON");

	assert_eq!(response_value["request_id"], request_id.to_string());
}

#[test]
fn inject_request_id_into_json_body_skips_non_object() {
	let request_id = Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").expect("valid uuid");
	let body = serde_json::json!(["a", "b", "c"]).to_string();

	assert!(routes::inject_request_id_into_json_body(body.as_bytes(), &request_id).is_none());
}
