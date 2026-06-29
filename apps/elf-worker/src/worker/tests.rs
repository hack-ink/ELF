use serde_json;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::worker::{self};

#[test]
fn pooled_vector_is_mean_of_chunks() {
	let chunks = vec![vec![1.0_f32, 3.0_f32], vec![3.0_f32, 5.0_f32]];
	let pooled = worker::mean_pool(&chunks).expect("Expected pooled vector.");

	assert_eq!(pooled, vec![2.0_f32, 4.0_f32]);
}

#[test]
fn project_doc_ref_fields_falls_back_to_created_at_timestamp() {
	let created_at = OffsetDateTime::parse("2025-01-01T00:00:00Z", &Rfc3339)
		.expect("Failed to parse fallback timestamp.");
	let (doc_ts, thread_id, domain, repo) = worker::project_doc_ref_fields(
		&serde_json::json!({"thread_id": ""}),
		created_at,
		"knowledge",
	)
	.expect("Expected projection.");

	assert_eq!(doc_ts, created_at.format(&Rfc3339).expect("Failed to format fallback doc_ts."));
	assert!(thread_id.is_none());
	assert!(domain.is_none());
	assert!(repo.is_none());
}

#[test]
fn project_doc_ref_fields_prefers_source_ref_ts() {
	let created_at = OffsetDateTime::parse("2025-01-01T00:00:00Z", &Rfc3339)
		.expect("Failed to parse fallback timestamp.");
	let source_ref = serde_json::json!({
		"ts": "2025-01-01T01:02:03Z",
		"doc_ts": "2020-01-01T00:00:00Z",
		"thread_id": "thread-42",
		"domain": "example.com",
		"repo": "org/repo"
	});
	let (doc_ts, thread_id, domain, repo) =
		worker::project_doc_ref_fields(&source_ref, created_at, "chat")
			.expect("Expected projection.");

	assert_eq!(doc_ts, "2025-01-01T01:02:03Z");
	assert_eq!(thread_id.as_deref(), Some("thread-42"));
	assert!(domain.is_none());
	assert!(repo.is_none());
}

#[test]
fn project_doc_ref_fields_uses_legacy_doc_ts_when_ts_is_missing() {
	let created_at = OffsetDateTime::parse("2025-01-01T00:00:00Z", &Rfc3339)
		.expect("Failed to parse fallback timestamp.");
	let source_ref = serde_json::json!({
		"doc_ts": "2025-01-01T02:03:04Z",
		"thread_id": "legacy-thread",
		"domain": "legacy.example",
		"repo": "legacy/repo"
	});
	let (doc_ts, thread_id, domain, repo) =
		worker::project_doc_ref_fields(&source_ref, created_at, "knowledge")
			.expect("Expected projection.");

	assert_eq!(doc_ts, "2025-01-01T02:03:04Z");
	assert!(thread_id.is_none());
	assert!(domain.is_none());
	assert!(repo.is_none());
}

#[test]
fn project_doc_ref_fields_gates_optional_ref_fields_by_doc_type() {
	let created_at = OffsetDateTime::parse("2025-01-01T00:00:00Z", &Rfc3339)
		.expect("Failed to parse fallback timestamp.");
	let source_ref = serde_json::json!({
		"thread_id": "thread-42",
		"domain": "example.com",
		"repo": "org/repo",
	});
	let (doc_ts_for_knowledge, thread_id_knowledge, domain_knowledge, repo_knowledge) =
		worker::project_doc_ref_fields(&source_ref, created_at, "knowledge")
			.expect("Expected projection.");

	assert_eq!(
		doc_ts_for_knowledge,
		created_at.format(&Rfc3339).expect("Failed to format fallback doc_ts.")
	);
	assert!(thread_id_knowledge.is_none());
	assert!(domain_knowledge.is_none());
	assert!(repo_knowledge.is_none());

	let chat_projection = worker::project_doc_ref_fields(&source_ref, created_at, "chat")
		.expect("Expected projection.");

	assert_eq!(chat_projection.1.as_deref(), Some("thread-42"));
	assert!(chat_projection.2.is_none());
	assert!(chat_projection.3.is_none());

	let search_projection = worker::project_doc_ref_fields(&source_ref, created_at, "search")
		.expect("Expected projection.");

	assert!(search_projection.1.is_none());
	assert_eq!(search_projection.2.as_deref(), Some("example.com"));
	assert!(search_projection.3.is_none());

	let dev_projection = worker::project_doc_ref_fields(&source_ref, created_at, "dev")
		.expect("Expected projection.");

	assert!(dev_projection.1.is_none());
	assert!(dev_projection.2.is_none());
	assert_eq!(dev_projection.3.as_deref(), Some("org/repo"));
}
