use crate::docs::{self, DocType, DocsPutRequest, Error};

#[test]
fn validate_docs_put_accepts_source_library_article_metadata() {
	let validated = docs::validate_docs_put(&DocsPutRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: Some(DocType::Knowledge.as_str().to_string()),
		title: Some("Saved article".to_string()),
		write_policy: None,
		source_ref: serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
			"source_kind": "article",
			"canonical_uri": "https://example.com/research/source-library",
			"captured_at": "2026-02-25T12:10:00Z",
			"source_created_at": "2026-02-24T09:00:00Z",
			"trust_label": "public_web",
			"author": "Example Author",
			"handle": "example-author",
			"excerpt_locator": {
				"quote": {
					"exact": "Source libraries preserve long-form evidence."
				},
				"position": {
					"start": 0,
					"end": 48
				}
			}
		}),
		content:
			"Source libraries preserve long-form evidence. Agents can hydrate exact excerpts later."
				.to_string(),
	})
	.expect("Expected source library metadata to be accepted.");

	assert_eq!(validated.doc_type, DocType::Knowledge);
}

#[test]
fn validate_docs_put_rejects_incomplete_source_library_metadata() {
	let err = docs::validate_docs_put(&DocsPutRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: Some(DocType::Knowledge.as_str().to_string()),
		title: Some("Saved article".to_string()),
		write_policy: None,
		source_ref: serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
			"source_kind": "article",
			"captured_at": "2026-02-25T12:10:00Z",
			"trust_label": "public_web"
		}),
		content: "Source libraries preserve long-form evidence.".to_string(),
	})
	.expect_err("Expected canonical_uri to be required for source library metadata.");

	match err {
		Error::InvalidRequest { message } => assert!(message.contains("canonical_uri")),
		other => panic!("Unexpected error: {other:?}"),
	}

	let err = docs::validate_docs_put(&DocsPutRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: Some(DocType::Knowledge.as_str().to_string()),
		title: Some("Saved thread".to_string()),
		write_policy: None,
		source_ref: serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
			"source_kind": "social_thread",
			"canonical_uri": "https://example.com/thread/123",
			"captured_at": "2026-02-25T12:10:00Z",
			"trust_label": "public_web"
		}),
		content: "The thread says source libraries need social captures.".to_string(),
	})
	.expect_err("Expected social_thread source_kind to require chat doc_type.");

	match err {
		Error::InvalidRequest { message } => assert!(message.contains("requires doc_type=chat")),
		other => panic!("Unexpected error: {other:?}"),
	}
}
