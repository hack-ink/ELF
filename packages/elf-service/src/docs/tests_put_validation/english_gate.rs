use crate::docs::{self, DocsPutRequest, Error};

#[test]
fn validate_docs_put_allows_doc_source_ref_v1_and_rejects_free_text() {
	docs::validate_docs_put(&DocsPutRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: None,
		title: Some("English title".to_string()),
		write_policy: None,
		source_ref: serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
			"notes": "English only."
		}),
		content: "English content.".to_string(),
	})
	.expect("Expected doc_source_ref/v1 source_ref to be accepted.");

	let err = docs::validate_docs_put(&DocsPutRequest {
		source_ref: serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
			"notes": "\u{4f60}\u{597d}\u{4e16}\u{754c}"
		}),
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: None,
		title: Some("English title".to_string()),
		write_policy: None,
		content: "English content.".to_string(),
	})
	.expect_err("Expected non-English free-text in source_ref.");

	match err {
		Error::NonEnglishInput { field } => assert_eq!(field, "$.source_ref[\"notes\"]"),
		other => panic!("Unexpected error: {other:?}"),
	}

	let err = docs::validate_docs_put(&DocsPutRequest {
		source_ref: serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
			"ref": "\u{4f60}\u{597d}\u{4e16}\u{754c}"
		}),
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: None,
		title: Some("English title".to_string()),
		write_policy: None,
		content: "English content.".to_string(),
	})
	.expect_err("Expected identifier lane with non-Latin text to be rejected.");

	match err {
		Error::NonEnglishInput { field } => assert_eq!(field, "$.source_ref[\"ref\"]"),
		other => panic!("Unexpected error: {other:?}"),
	}
}
