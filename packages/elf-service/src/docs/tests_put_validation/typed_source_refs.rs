use crate::docs::{self, DocType, DocsPutRequest, Error};

#[test]
fn validate_docs_put_rejects_chat_source_ref_with_missing_thread_metadata() {
	let err = docs::validate_docs_put(&DocsPutRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: Some(DocType::Chat.as_str().to_string()),
		title: None,
		write_policy: None,
		source_ref: serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "chat",
			"ts": "2026-02-25T12:00:00Z",
		}),
		content: "Hello world.".to_string(),
	})
	.expect_err("Expected chat source_ref to require thread_id/role.");

	match err {
		Error::InvalidRequest { message } => assert!(message.contains("thread_id")),
		other => panic!("Unexpected error: {other:?}"),
	}
}

#[test]
fn validate_docs_put_rejects_search_source_ref_with_missing_domain() {
	let err = docs::validate_docs_put(&DocsPutRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: Some(DocType::Search.as_str().to_string()),
		title: None,
		write_policy: None,
		source_ref: serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "search",
			"ts": "2026-02-25T12:00:00Z",
			"query": "test",
			"url": "https://example.com",
		}),
		content: "Hello world.".to_string(),
	})
	.expect_err("Expected search source_ref to require domain.");

	match err {
		Error::InvalidRequest { message } => assert!(message.contains("domain")),
		other => panic!("Unexpected error: {other:?}"),
	}
}

#[test]
fn validate_docs_put_rejects_dev_source_ref_with_multiple_identifiers() {
	let err = docs::validate_docs_put(&DocsPutRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: Some(DocType::Dev.as_str().to_string()),
		title: None,
		write_policy: None,
		source_ref: serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "dev",
			"ts": "2026-02-25T12:00:00Z",
			"repo": "hack-ink/ELF",
			"commit_sha": "9f0a3f4c4eb58bfcf4a5f4f9d0c7be0e13c2f8d19",
			"issue_number": 123,
		}),
		content: "Hello world.".to_string(),
	})
	.expect_err("Expected dev source_ref to enforce exactly one identifier field.");

	match err {
		Error::InvalidRequest { message } => {
			assert!(message.contains("exactly one of commit_sha, pr_number, or issue_number"))
		},
		other => panic!("Unexpected error: {other:?}"),
	}
}

#[test]
fn validate_docs_put_uses_source_ref_doc_type_when_request_doc_type_is_absent() {
	let resolved_doc_type = docs::validate_docs_put(&DocsPutRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: None,
		title: None,
		write_policy: None,
		source_ref: serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "chat",
			"ts": "2026-02-25T12:00:00Z",
			"thread_id": "thread-1",
			"role": "assistant"
		}),
		content: "Hello world.".to_string(),
	})
	.expect("Expected valid source_ref to resolve doc_type.");

	assert_eq!(resolved_doc_type.doc_type, DocType::Chat);
}
