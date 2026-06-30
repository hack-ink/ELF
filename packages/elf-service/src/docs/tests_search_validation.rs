use qdrant_client::qdrant::{
	DatetimeRange, Filter, condition::ConditionOneOf, r#match::MatchValue,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::docs::{
	self, DocType, DocsSearchL0Filters, DocsSearchL0Request, DocsSparseMode, Error,
	tests::{self, PROJECT_ID, TENANT_ID},
};

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
fn validate_docs_search_l0_defaults_status_and_filters_dates() {
	let filters = docs::validate_docs_search_l0(&tests::test_request_with_query("hello world"))
		.expect("valid request");

	assert_eq!(filters.status, "active");

	let bad_dates = DocsSearchL0Request {
		updated_after: Some("2026-02-25T12:00:00Z".to_string()),
		updated_before: Some("2026-02-25T11:00:00Z".to_string()),
		sparse_mode: None,
		domain: None,
		repo: None,
		..tests::test_request_with_query("status")
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
	let filter = docs::build_doc_search_filter(
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
	let filters = docs::validate_docs_search_l0(&tests::test_request_with_query("status"))
		.expect("valid request");

	assert!(matches!(filters.sparse_mode, DocsSparseMode::Auto));
}

#[test]
fn should_enable_sparse_auto_uses_symbol_cues() {
	assert!(docs::should_enable_sparse_auto("https://example.com/search?q=abc"));
	assert!(!docs::should_enable_sparse_auto("how to debug a timeout"));
}
