mod forwarding;
mod schemas;
mod tool_definitions;

use axum::http::HeaderMap;

use crate::app::{
	McpAuthState,
	server::{ElfContextHeaders, ElfMcp, HEADER_AUTHORIZATION},
};
use elf_config::McpContext;

#[test]
fn admin_paths_use_admin_api_base() {
	let context = McpContext {
		tenant_id: "tenant-a".to_string(),
		project_id: "project-a".to_string(),
		agent_id: "agent-a".to_string(),
		read_profile: "private_plus_project".to_string(),
	};
	let mcp = ElfMcp::new(
		"http://127.0.0.1:9000".to_string(),
		"http://127.0.0.1:9001".to_string(),
		ElfContextHeaders::new(&context),
		McpAuthState::Off,
	);

	assert_eq!(mcp.api_base_for_path("/v2/admin/traces/recent"), "http://127.0.0.1:9001");
	assert_eq!(mcp.api_base_for_path("/v2/admin/notes/abcd/provenance"), "http://127.0.0.1:9001");
	assert_eq!(mcp.api_base_for_path("/v2/admin/notes/abcd/history"), "http://127.0.0.1:9001");
	assert_eq!(mcp.api_base_for_path("/v2/searches"), "http://127.0.0.1:9000");
	assert_eq!(mcp.api_base_for_path("/v2/recall-debug/panel"), "http://127.0.0.1:9000");
}

#[test]
fn off_mode_allows_requests_without_auth_header() {
	let headers = HeaderMap::new();

	assert!(super::is_authorized(&headers, &McpAuthState::Off));
}

#[test]
fn static_keys_mode_requires_authorization_bearer_header() {
	let mut headers = HeaderMap::new();

	headers.insert(HEADER_AUTHORIZATION, "Bearer token-a".parse().expect("valid header"));

	assert!(super::is_authorized(
		&headers,
		&McpAuthState::StaticKeys { bearer_token: "token-a".to_string() }
	));
}

#[test]
fn static_keys_mode_rejects_non_bearer_schemes() {
	let mut headers = HeaderMap::new();

	headers.insert(HEADER_AUTHORIZATION, "bearer token-a".parse().expect("valid header"));

	assert!(!super::is_authorized(
		&headers,
		&McpAuthState::StaticKeys { bearer_token: "token-a".to_string() }
	));
}
