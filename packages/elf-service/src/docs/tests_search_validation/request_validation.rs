use crate::docs::{
	self, DocsSearchL0Request, Error,
	tests::{PROJECT_ID, TENANT_ID},
};

#[test]
fn docs_search_l0_requires_chat_doc_type_for_thread_id() {
	let err = docs::validate_docs_search_l0(&DocsSearchL0Request {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		caller_agent_id: "agent".to_string(),
		read_profile: "private_plus_project".to_string(),
		query: "thread".to_string(),
		scope: None,
		status: None,
		doc_type: Some("search".to_string()),
		sparse_mode: None,
		domain: None,
		repo: None,
		agent_id: None,
		thread_id: Some("thread-1".to_string()),
		updated_after: None,
		updated_before: None,
		ts_gte: None,
		ts_lte: None,
		top_k: None,
		candidate_k: None,
		explain: None,
	})
	.expect_err("Expected thread_id to require doc_type=chat.");

	match err {
		Error::InvalidRequest { message } => assert!(message.contains("thread_id requires")),
		other => panic!("Unexpected error: {other:?}"),
	}

	docs::validate_docs_search_l0(&DocsSearchL0Request {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		caller_agent_id: "agent".to_string(),
		read_profile: "private_plus_project".to_string(),
		query: "thread".to_string(),
		scope: None,
		status: None,
		doc_type: Some("chat".to_string()),
		sparse_mode: None,
		domain: None,
		repo: None,
		agent_id: None,
		thread_id: Some("thread-1".to_string()),
		updated_after: None,
		updated_before: None,
		ts_gte: None,
		ts_lte: None,
		top_k: None,
		candidate_k: None,
		explain: None,
	})
	.expect("Expected thread_id filter to be accepted for chat.");
}

#[test]
fn validate_docs_search_l0_rejects_invalid_status() {
	let err = docs::validate_docs_search_l0(&DocsSearchL0Request {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		caller_agent_id: "agent".to_string(),
		read_profile: "private_plus_project".to_string(),
		query: "status".to_string(),
		scope: None,
		status: Some("archived".to_string()),
		doc_type: None,
		sparse_mode: None,
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
	.expect_err("Expected invalid status to be rejected.");

	match err {
		Error::InvalidRequest { message } => assert!(message.contains("status")),
		other => panic!("Unexpected error: {other:?}"),
	}
}

#[test]
fn validate_docs_search_l0_rejects_invalid_datetime_format() {
	let err = docs::validate_docs_search_l0(&DocsSearchL0Request {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		caller_agent_id: "agent".to_string(),
		read_profile: "private_plus_project".to_string(),
		query: "status".to_string(),
		scope: None,
		status: None,
		doc_type: None,
		sparse_mode: None,
		domain: None,
		repo: None,
		agent_id: None,
		thread_id: None,
		updated_after: Some("2026-02-25T12:00:00".to_string()),
		updated_before: None,
		ts_gte: None,
		ts_lte: None,
		top_k: None,
		candidate_k: None,
		explain: None,
	})
	.expect_err("Expected invalid RFC3339 datetime to be rejected.");

	match err {
		Error::InvalidRequest { message } => assert!(message.contains("RFC3339")),
		other => panic!("Unexpected error: {other:?}"),
	}
}

#[test]
fn validate_docs_search_l0_rejects_invalid_doc_ts_order() {
	let err = docs::validate_docs_search_l0(&DocsSearchL0Request {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		caller_agent_id: "agent".to_string(),
		read_profile: "private_plus_project".to_string(),
		query: "status".to_string(),
		scope: None,
		status: None,
		doc_type: None,
		sparse_mode: None,
		domain: None,
		repo: None,
		agent_id: None,
		thread_id: None,
		updated_after: None,
		updated_before: None,
		ts_gte: Some("2026-02-25T12:00:00Z".to_string()),
		ts_lte: Some("2026-02-25T11:00:00Z".to_string()),
		top_k: None,
		candidate_k: None,
		explain: None,
	})
	.expect_err("Expected bad doc_ts order to be rejected.");

	match err {
		Error::InvalidRequest { message } => {
			assert!(message.contains("earlier"));
		},
		other => panic!("Unexpected error: {other:?}"),
	}
}

#[test]
fn validate_docs_search_l0_rejects_domain_without_doc_type_search() {
	let err = docs::validate_docs_search_l0(&DocsSearchL0Request {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		caller_agent_id: "agent".to_string(),
		read_profile: "private_plus_project".to_string(),
		query: "status".to_string(),
		scope: None,
		status: None,
		doc_type: None,
		sparse_mode: None,
		domain: Some("example.com".to_string()),
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
	.expect_err("Expected domain without doc_type=search to be rejected.");

	match err {
		Error::InvalidRequest { message } => {
			assert!(message.contains("doc_type=search"));
		},
		other => panic!("Unexpected error: {other:?}"),
	}
}

#[test]
fn validate_docs_search_l0_rejects_repo_without_doc_type_dev() {
	let err = docs::validate_docs_search_l0(&DocsSearchL0Request {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		caller_agent_id: "agent".to_string(),
		read_profile: "private_plus_project".to_string(),
		query: "status".to_string(),
		scope: None,
		status: None,
		doc_type: None,
		sparse_mode: None,
		domain: None,
		repo: Some("hack-ink/ELF".to_string()),
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
	.expect_err("Expected repo without doc_type=dev to be rejected.");

	match err {
		Error::InvalidRequest { message } => {
			assert!(message.contains("doc_type=dev"));
		},
		other => panic!("Unexpected error: {other:?}"),
	}
}
