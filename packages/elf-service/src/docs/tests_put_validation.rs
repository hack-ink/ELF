use crate::docs::{self, DocType, DocsPutRequest, Error};
use elf_domain::writegate::{WritePolicy, WritePolicyAudit, WriteSpan};

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

#[test]
fn validate_docs_put_applies_write_policy_and_includes_audit() {
	let validated = docs::validate_docs_put(&DocsPutRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: Some(DocType::Knowledge.as_str().to_string()),
		title: None,
		write_policy: Some(WritePolicy {
			exclusions: vec![WriteSpan { start: 6, end: 35 }],
			redactions: vec![],
		}),
		source_ref: serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
		}),
		content: "Hello sk-abcdefghijklmnopqrstuvwxyz!".to_string(),
	})
	.expect("Expected valid write policy transformation.");
	let expected_audit = WritePolicyAudit {
		exclusions: vec![WriteSpan { start: 6, end: 35 }],
		..Default::default()
	};

	assert_eq!(validated.content, "Hello !".to_string());
	assert_eq!(validated.write_policy_audit.unwrap_or_default(), expected_audit);
}

#[test]
fn validate_docs_put_rejects_secret_after_write_policy() {
	let err = docs::validate_docs_put(&DocsPutRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "project_shared".to_string(),
		doc_type: Some(DocType::Knowledge.as_str().to_string()),
		title: None,
		write_policy: Some(WritePolicy { exclusions: vec![], redactions: vec![] }),
		source_ref: serde_json::json!({
			"schema": "doc_source_ref/v1",
			"doc_type": "knowledge",
			"ts": "2026-02-25T12:00:00Z",
		}),
		content: "Hello sk-abcdefghijklmnopqrstuvwxyz!".to_string(),
	})
	.expect_err("Expected secret-bearing content to be rejected.");

	match err {
		Error::InvalidRequest { message } => assert!(message.contains("contains secrets")),
		other => panic!("Unexpected error: {other:?}"),
	}
}

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
