use crate::docs::{self, DocType, DocsPutRequest, Error};

#[test]
fn validate_docs_put_rejects_invalid_doc_type() {
	let err = docs::validate_docs_put(&DocsPutRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: None,
		title: None,
		write_policy: None,
		source_ref: serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "invalid",
			"ts": "2026-02-25T12:00:00Z",
		}),
		content: "Hello world.".to_string(),
	})
	.expect_err("Expected invalid doc_type to be rejected.");

	match err {
		Error::InvalidRequest { message } => assert!(message.contains("doc_type")),
		other => panic!("Unexpected error: {other:?}"),
	}
}

#[test]
fn validate_docs_put_rejects_missing_source_ref() {
	let err = docs::validate_docs_put(&DocsPutRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: Some(DocType::Knowledge.as_str().to_string()),
		title: None,
		write_policy: None,
		source_ref: serde_json::json!({"schema":"doc_source_ref/v1", "doc_type":"knowledge"}),
		content: "Hello world.".to_string(),
	})
	.expect_err("Expected missing source_ref.ts to be rejected.");

	match err {
		Error::InvalidRequest { message } => assert!(message.contains("source_ref[\"ts\"]")),
		other => panic!("Unexpected error: {other:?}"),
	}
}

#[test]
fn validate_docs_put_rejects_non_object_source_ref() {
	let err = docs::validate_docs_put(&DocsPutRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: None,
		title: None,
		write_policy: None,
		source_ref: serde_json::json!("legacy-shape"),
		content: "Hello world.".to_string(),
	})
	.expect_err("Expected non-object source_ref to be rejected.");

	match err {
		Error::InvalidRequest { message } => {
			assert!(message.contains("source_ref must be a JSON object"))
		},
		other => panic!("Unexpected error: {other:?}"),
	}
}

#[test]
fn validate_docs_put_rejects_mismatched_request_and_source_ref_doc_type() {
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
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
		}),
		content: "Hello world.".to_string(),
	})
	.expect_err("Expected mismatched doc_type to be rejected.");

	match err {
		Error::InvalidRequest { message } => assert!(message.contains("match")),
		other => panic!("Unexpected error: {other:?}"),
	}
}

#[test]
fn validate_docs_put_rejects_wrong_source_ref_schema() {
	let err = docs::validate_docs_put(&DocsPutRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: None,
		title: None,
		write_policy: None,
		source_ref: serde_json::json!({
			"schema": "note_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
		}),
		content: "Hello world.".to_string(),
	})
	.expect_err("Expected wrong source_ref.schema to be rejected.");

	match err {
		Error::InvalidRequest { message } => assert!(message.contains("doc_source_ref/v1")),
		other => panic!("Unexpected error: {other:?}"),
	}
}
