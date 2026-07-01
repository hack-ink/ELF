use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::docs::{
	self, DocType, DocsSearchL0Filters, DocsSearchL0Request, DocsSparseMode, Error,
	tests::{self, PROJECT_ID, TENANT_ID, tests_search_validation::support},
};

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

	assert_eq!(support::first_match_value(&filter, "tenant_id").as_deref(), Some("tenant"));
	assert_eq!(support::first_match_value(&filter, "status").as_deref(), Some("deleted"));
	assert_eq!(support::first_match_value(&filter, "scope").as_deref(), Some("project_shared"));
	assert_eq!(support::first_match_value(&filter, "doc_type").as_deref(), Some("chat"));
	assert_eq!(support::first_match_value(&filter, "agent_id").as_deref(), Some("owner"));
	assert_eq!(support::first_match_value(&filter, "thread_id").as_deref(), Some("thread-7"));
	assert_eq!(support::first_match_value(&filter, "domain").as_deref(), None);
	assert_eq!(support::first_match_value(&filter, "repo").as_deref(), None);

	let datetime_range = support::first_datetime_range(&filter, "updated_at")
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

	let doc_ts_range = support::first_datetime_range(&filter, "doc_ts")
		.expect("Expected datetime filter for doc_ts.");
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
