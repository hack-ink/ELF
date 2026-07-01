use crate::docs::{
	self, DocsSearchL0Request, DocsSparseMode, Error,
	tests::{self, PROJECT_ID, TENANT_ID},
};

#[test]
fn validate_docs_search_l0_rejects_invalid_sparse_mode() {
	let err = docs::validate_docs_search_l0(&DocsSearchL0Request {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		caller_agent_id: "agent".to_string(),
		read_profile: "private_plus_project".to_string(),
		query: "status".to_string(),
		scope: None,
		status: None,
		doc_type: None,
		sparse_mode: Some("invalid".to_string()),
		domain: None,
		repo: None,
		agent_id: None,
		thread_id: None,
		updated_after: None,
		updated_before: None,
		ts_gte: None,
		ts_lte: None,
		top_k: None,
		candidate_k: None,
		explain: None,
	})
	.expect_err("Expected invalid sparse mode to be rejected.");

	match err {
		Error::InvalidRequest { message } => {
			assert!(message.contains("sparse_mode"));
		},
		other => panic!("Unexpected error: {other:?}"),
	}
}

#[test]
fn validate_docs_search_l0_default_sparse_mode() {
	let filters = docs::validate_docs_search_l0(&tests::test_request_with_query("status"))
		.expect("valid request");

	assert!(matches!(filters.sparse_mode, DocsSparseMode::Auto));
}

#[test]
fn should_enable_sparse_auto_uses_symbol_cues() {
	assert!(docs::should_enable_sparse_auto("https://example.com/search?q=abc"));
	assert!(!docs::should_enable_sparse_auto("how to debug a timeout"));
}
