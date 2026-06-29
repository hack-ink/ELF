use ahash::AHashMap;
use qdrant_client::qdrant::{
	DatetimeRange, Filter, condition::ConditionOneOf, r#match::MatchValue,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokenizers::{Tokenizer, models::wordlevel::WordLevel, pre_tokenizers::whitespace::Whitespace};
use uuid::Uuid;

use crate::docs::{
	self, DocType, DocsPutRequest, DocsSearchL0Filters, DocsSearchL0Request, DocsSparseMode, Error,
};
use elf_domain::writegate::{WritePolicy, WritePolicyAudit, WriteRedactionResult, WriteSpan};
use elf_storage::models::DocChunk;

const TENANT_ID: &str = "tenant";
const PROJECT_ID: &str = "project";

fn test_request_with_query(query: &str) -> DocsSearchL0Request {
	DocsSearchL0Request {
		tenant_id: TENANT_ID.to_string(),
		project_id: PROJECT_ID.to_string(),
		caller_agent_id: "agent".to_string(),
		read_profile: "private_plus_project".to_string(),
		query: query.to_string(),
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
		ts_gte: None,
		ts_lte: None,
		top_k: None,
		candidate_k: None,
		explain: None,
	}
}

fn first_datetime_range(filter: &Filter, key: &str) -> Option<DatetimeRange> {
	for condition in &filter.must {
		if let Some(ConditionOneOf::Field(field)) = condition.condition_one_of.as_ref() {
			if field.key != key {
				continue;
			}

			if let Some(range) = field.datetime_range.as_ref() {
				return Some(*range);
			}
		}
	}

	None
}

fn first_match_value(filter: &Filter, key: &str) -> Option<String> {
	for condition in &filter.must {
		if let Some(ConditionOneOf::Field(field)) = condition.condition_one_of.as_ref() {
			if field.key != key {
				continue;
			}

			if let Some(r#match) = field.r#match.as_ref() {
				let Some(match_value) = r#match.match_value.as_ref() else {
					continue;
				};

				return match match_value {
					MatchValue::Keyword(value) => Some(value.clone()),
					_ => None,
				};
			}
		}
	}

	None
}

fn test_tokenizer() -> Tokenizer {
	let mut vocab = AHashMap::new();

	vocab.insert("alpha".to_string(), 1_u32);
	vocab.insert("beta".to_string(), 2_u32);
	vocab.insert("charlie".to_string(), 3_u32);
	vocab.insert("delta".to_string(), 4_u32);
	vocab.insert("<unk>".to_string(), 0_u32);

	let model = WordLevel::builder()
		.vocab(vocab)
		.unk_token("<unk>".to_string())
		.build()
		.expect("Failed to build test tokenizer.");
	let mut tokenizer = Tokenizer::new(model);

	tokenizer.with_pre_tokenizer(Some(Whitespace));

	tokenizer
}

#[test]
fn doc_type_parses_and_serializes() {
	let encoded =
		serde_json::to_string(&DocType::Knowledge).expect("Expected DocType serialization.");
	let parsed =
		serde_json::from_str::<DocType>("\"knowledge\"").expect("Expected parse to succeed.");
	let invalid: Result<DocType, _> = serde_json::from_str("\"invalid\"");

	assert_eq!(encoded, "\"knowledge\"");
	assert_eq!(parsed, DocType::Knowledge);
	assert!(invalid.is_err());
}

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
fn resolve_doc_chunking_profile_is_deterministic_by_doc_type() {
	let small = docs::resolve_doc_chunking_profile(DocType::Chat);

	assert_eq!(small.max_tokens, 1_024);
	assert_eq!(small.overlap_tokens, 128);

	let default = docs::resolve_doc_chunking_profile(DocType::Knowledge);

	assert_eq!(default.max_tokens, 2_048);
	assert_eq!(default.overlap_tokens, 256);
}

#[test]
fn validate_docs_search_l0_defaults_status_and_filters_dates() {
	let filters = docs::validate_docs_search_l0(&test_request_with_query("hello world"))
		.expect("valid request");

	assert_eq!(filters.status, "active");

	let bad_dates = DocsSearchL0Request {
		updated_after: Some("2026-02-25T12:00:00Z".to_string()),
		updated_before: Some("2026-02-25T11:00:00Z".to_string()),
		sparse_mode: None,
		domain: None,
		repo: None,
		..test_request_with_query("status")
	};
	let err = docs::validate_docs_search_l0(&bad_dates)
		.expect_err("Expected bad date order to be rejected.");

	match err {
		Error::InvalidRequest { message } => {
			assert!(message.contains("earlier"));
		},
		other => panic!("Unexpected error: {other:?}"),
	}
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
fn build_doc_search_filter_applies_status_and_requested_filters() {
	let filters = DocsSearchL0Filters {
		scope: Some("project_shared".to_string()),
		status: "deleted".to_string(),
		doc_type: Some(DocType::Chat),
		sparse_mode: DocsSparseMode::Auto,
		domain: None,
		repo: None,
		agent_id: Some("owner".to_string()),
		thread_id: Some("thread-7".to_string()),
		updated_after: Some(
			OffsetDateTime::parse("2026-02-20T00:00:00Z", &Rfc3339).expect("Invalid timestamp."),
		),
		updated_before: Some(
			OffsetDateTime::parse("2026-02-28T00:00:00Z", &Rfc3339).expect("Invalid timestamp."),
		),
		ts_gte: Some(
			OffsetDateTime::parse("2026-01-01T00:00:00Z", &Rfc3339).expect("Invalid timestamp."),
		),
		ts_lte: Some(
			OffsetDateTime::parse("2026-12-31T00:00:00Z", &Rfc3339).expect("Invalid timestamp."),
		),
	};
	let filter = super::build_doc_search_filter(
		TENANT_ID,
		PROJECT_ID,
		"requester",
		&["agent_private".to_string(), "project_shared".to_string()],
		&filters,
	);

	assert_eq!(first_match_value(&filter, "tenant_id").as_deref(), Some("tenant"));
	assert_eq!(first_match_value(&filter, "status").as_deref(), Some("deleted"));
	assert_eq!(first_match_value(&filter, "scope").as_deref(), Some("project_shared"));
	assert_eq!(first_match_value(&filter, "doc_type").as_deref(), Some("chat"));
	assert_eq!(first_match_value(&filter, "agent_id").as_deref(), Some("owner"));
	assert_eq!(first_match_value(&filter, "thread_id").as_deref(), Some("thread-7"));
	assert_eq!(first_match_value(&filter, "domain").as_deref(), None);
	assert_eq!(first_match_value(&filter, "repo").as_deref(), None);

	let datetime_range = first_datetime_range(&filter, "updated_at")
		.expect("Expected datetime filter for updated_at.");
	let after =
		OffsetDateTime::parse("2026-02-20T00:00:00Z", &Rfc3339).expect("Invalid timestamp.");
	let before =
		OffsetDateTime::parse("2026-02-28T00:00:00Z", &Rfc3339).expect("Invalid timestamp.");
	let lt = datetime_range.lt.as_ref().expect("Expected datetime filter .lt value.");
	let gt = datetime_range.gt.as_ref().expect("Expected datetime filter .gt value.");

	assert_eq!(lt.seconds, before.unix_timestamp());
	assert_eq!(lt.nanos, before.nanosecond() as i32);
	assert_eq!(gt.seconds, after.unix_timestamp());
	assert_eq!(gt.nanos, after.nanosecond() as i32);
	assert!(datetime_range.gte.is_none());
	assert!(datetime_range.lte.is_none());

	let doc_ts_range =
		first_datetime_range(&filter, "doc_ts").expect("Expected datetime filter for doc_ts.");
	let gte = doc_ts_range.gte.as_ref().expect("Expected datetime filter .gte value.");
	let lte = doc_ts_range.lte.as_ref().expect("Expected datetime filter .lte value.");
	let doc_ts_gte =
		OffsetDateTime::parse("2026-01-01T00:00:00Z", &Rfc3339).expect("Invalid timestamp.");
	let doc_ts_lte =
		OffsetDateTime::parse("2026-12-31T00:00:00Z", &Rfc3339).expect("Invalid timestamp.");

	assert_eq!(gte.seconds, doc_ts_gte.unix_timestamp());
	assert_eq!(gte.nanos, doc_ts_gte.nanosecond() as i32);
	assert_eq!(lte.seconds, doc_ts_lte.unix_timestamp());
	assert_eq!(lte.nanos, doc_ts_lte.nanosecond() as i32);
	assert!(doc_ts_range.gt.is_none());
	assert!(doc_ts_range.lt.is_none());
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

#[test]
fn validate_docs_search_l0_default_sparse_mode() {
	let filters =
		docs::validate_docs_search_l0(&test_request_with_query("status")).expect("valid request");

	assert!(matches!(filters.sparse_mode, DocsSparseMode::Auto));
}

#[test]
fn should_enable_sparse_auto_uses_symbol_cues() {
	assert!(super::should_enable_sparse_auto("https://example.com/search?q=abc"));
	assert!(!super::should_enable_sparse_auto("how to debug a timeout"));
}

#[test]
fn excerpt_level_max_supports_l0_and_rejects_unknown_level() {
	assert_eq!(
		super::excerpt_level_max("L0").expect("Expected L0 to be supported."),
		super::DEFAULT_L0_MAX_BYTES
	);
	assert!(super::excerpt_level_max("L3").is_err());
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
fn source_capture_metadata_uses_stable_record_and_span_ids() {
	let now = OffsetDateTime::parse("2026-02-25T12:15:00Z", &Rfc3339)
		.expect("Expected test timestamp to parse.");
	let source_ref = serde_json::json!({
		"schema": "doc_source_ref/v1",
		"doc_type": "knowledge",
		"ts": "2026-02-25T12:00:00Z",
		"source_kind": "article",
		"canonical_uri": "https://example.com/research/source-library",
		"captured_at": "2026-02-25T12:10:00Z",
		"trust_label": "public_web",
	});
	let source_ref = source_ref.as_object().expect("Expected source_ref object.");
	let content_hash = "doc-content-hash";
	let doc_id = super::source_record_id_for(
		TENANT_ID,
		PROJECT_ID,
		"owner",
		"project_shared",
		DocType::Knowledge,
		source_ref,
		content_hash,
	);
	let repeated_doc_id = super::source_record_id_for(
		TENANT_ID,
		PROJECT_ID,
		"owner",
		"project_shared",
		DocType::Knowledge,
		source_ref,
		content_hash,
	);
	let chunk_id = super::doc_chunk_id_for(doc_id, 0);
	let chunk = DocChunk {
		chunk_id,
		doc_id,
		chunk_index: 0,
		start_offset: 0,
		end_offset: 42,
		chunk_text: "Source libraries preserve long-form evidence.".to_string(),
		chunk_hash: "chunk-content-hash".to_string(),
		created_at: now,
	};
	let capture = super::build_source_capture_summary(super::SourceCaptureSummaryInput {
		doc_id,
		source_ref,
		doc_type: DocType::Knowledge,
		scope: "project_shared",
		title: Some("Saved article"),
		content_hash,
		raw_content_hash: "raw-content-hash",
		now,
		chunks: &[chunk],
		write_policy_audit: None,
	})
	.expect("Expected source capture summary.");

	assert_eq!(doc_id, repeated_doc_id);
	assert_eq!(capture.schema, "doc_source_capture/v1");
	assert_eq!(capture.source_record_id, doc_id);
	assert_eq!(capture.origin, "https://example.com/research/source-library");
	assert_eq!(capture.captured_at, "2026-02-25T12:10:00Z");
	assert_eq!(capture.content_hash, content_hash);
	assert_eq!(capture.visibility_scope, "project_shared");
	assert_eq!(capture.title.as_deref(), Some("Saved article"));
	assert_eq!(capture.source_type, "article");
	assert_eq!(capture.source_spans.len(), 1);
	assert_eq!(capture.source_spans[0].schema, "doc_source_span/v1");
	assert_eq!(capture.source_spans[0].chunk_id, Some(chunk_id));
	assert_eq!(capture.source_spans[0].status, "captured");
	assert_eq!(capture.source_spans[0].reason_code, None);
	assert_eq!(capture.source_spans[0].start_offset, 0);
	assert_eq!(capture.source_spans[0].end_offset, 42);
	assert_eq!(
		capture.source_spans[0].span_id,
		super::source_span_id(content_hash, 0, 42, "captured")
	);
}

#[test]
fn normalized_source_ref_records_policy_span_reasons() {
	let now = OffsetDateTime::parse("2026-02-25T12:15:00Z", &Rfc3339)
		.expect("Expected test timestamp to parse.");
	let source_ref = serde_json::json!({
		"schema": "doc_source_ref/v1",
		"doc_type": "knowledge",
		"ts": "2026-02-25T12:00:00Z",
		"uri": "file:///tmp/source.txt",
	});
	let source_ref_map = source_ref.as_object().expect("Expected source_ref object.");
	let audit = WritePolicyAudit {
		exclusions: vec![WriteSpan { start: 6, end: 12 }],
		redactions: vec![WriteRedactionResult {
			span: WriteSpan { start: 20, end: 30 },
			replacement: "[redacted]".to_string(),
		}],
	};
	let doc_id = super::source_record_id_for(
		TENANT_ID,
		PROJECT_ID,
		"owner",
		"project_shared",
		DocType::Knowledge,
		source_ref_map,
		"stored-hash",
	);
	let capture = super::build_source_capture_summary(super::SourceCaptureSummaryInput {
		doc_id,
		source_ref: source_ref_map,
		doc_type: DocType::Knowledge,
		scope: "project_shared",
		title: None,
		content_hash: "stored-hash",
		raw_content_hash: "raw-hash",
		now,
		chunks: &[],
		write_policy_audit: Some(&audit),
	})
	.expect("Expected source capture summary.");
	let normalized = super::normalize_source_ref_for_capture(source_ref, &capture)
		.expect("Expected normalized source_ref");

	assert_eq!(capture.policy_spans.len(), 2);
	assert_eq!(capture.policy_spans[0].status, "excluded");
	assert_eq!(capture.policy_spans[0].reason_code.as_deref(), Some("WRITE_POLICY_EXCLUSION"));
	assert_eq!(capture.policy_spans[1].status, "redacted");
	assert_eq!(capture.policy_spans[1].reason_code.as_deref(), Some("WRITE_POLICY_REDACTION"));
	assert_eq!(normalized["source_record_id"], doc_id.to_string());
	assert_eq!(normalized["origin"], "file:///tmp/source.txt");
	assert_eq!(normalized["captured_at"], "2026-02-25T12:15:00Z");
	assert_eq!(normalized["content_hash"], "stored-hash");
	assert_eq!(normalized["visibility_scope"], "project_shared");
	assert_eq!(normalized["source_type"], "knowledge");
	assert_eq!(normalized["policy_spans"][0]["reason_code"], "WRITE_POLICY_EXCLUSION");
	assert_eq!(normalized["policy_spans"][1]["reason_code"], "WRITE_POLICY_REDACTION");
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
fn docs_l0_pointer_carries_hashes_and_position_locator() {
	let now = OffsetDateTime::parse("2026-02-25T12:00:00Z", &Rfc3339)
		.expect("Expected test timestamp to parse.");
	let row = super::DocSearchRow {
		chunk_id: Uuid::parse_str("11111111-1111-4111-8111-111111111111")
			.expect("Expected chunk UUID."),
		doc_id: Uuid::parse_str("22222222-2222-4222-8222-222222222222")
			.expect("Expected doc UUID."),
		scope: "project_shared".to_string(),
		doc_type: "knowledge".to_string(),
		project_id: "project".to_string(),
		agent_id: "agent".to_string(),
		updated_at: now,
		content_hash: "doc-hash".to_string(),
		chunk_hash: "chunk-hash".to_string(),
		start_offset: 12,
		end_offset: 64,
		chunk_text: "Source libraries preserve long-form evidence.".to_string(),
	};
	let pointer = super::build_docs_l0_pointer(&row, row.chunk_id);

	assert_eq!(pointer.schema, "source_ref/v1");
	assert_eq!(pointer.resolver, "elf_doc_ext/v1");
	assert_eq!(pointer.hashes.content_hash, "doc-hash");
	assert_eq!(pointer.hashes.chunk_hash, "chunk-hash");
	assert_eq!(pointer.reference.source_record_id, row.doc_id);
	assert_eq!(pointer.reference.source_span_id, pointer.locator.span_id);
	assert_eq!(pointer.locator.position.start, 12);
	assert_eq!(pointer.locator.position.end, 64);
	assert_eq!(pointer.locator.span_id, super::source_span_id("doc-hash", 12, 64, "captured"));
	assert_eq!(pointer.state.content_hash, pointer.hashes.content_hash);
	assert_eq!(pointer.state.chunk_hash, pointer.hashes.chunk_hash);
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
	let expected_audit = elf_domain::writegate::WritePolicyAudit {
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

#[test]
fn split_tokens_by_offsets_preserves_original_substring_offsets() {
	let tokenizer = test_tokenizer();
	let chunks = super::split_tokens_by_offsets("alpha bravo charlie delta", 2, 1, 10, &tokenizer)
		.expect("Expected token chunking to succeed.");

	assert_eq!(chunks.len(), 3);
	assert_eq!(chunks[0].start_offset, 0);
	assert_eq!(chunks[0].end_offset, 11);
	assert_eq!(chunks[1].start_offset, 6);
	assert_eq!(chunks[1].end_offset, 19);
	assert_eq!(chunks[2].start_offset, 12);
	assert_eq!(chunks[2].end_offset, 25);

	for chunk in &chunks {
		assert_eq!(chunk.text, "alpha bravo charlie delta"[chunk.start_offset..chunk.end_offset]);
	}
}
