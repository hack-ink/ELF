use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::docs::{
	self, DocSearchRow, DocType, SourceCaptureSummaryInput,
	tests::{PROJECT_ID, TENANT_ID},
};
use elf_domain::writegate::{WritePolicyAudit, WriteRedactionResult, WriteSpan};
use elf_storage::models::DocChunk;

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
	let doc_id = docs::source_record_id_for(
		TENANT_ID,
		PROJECT_ID,
		"owner",
		"project_shared",
		DocType::Knowledge,
		source_ref,
		content_hash,
	);
	let repeated_doc_id = docs::source_record_id_for(
		TENANT_ID,
		PROJECT_ID,
		"owner",
		"project_shared",
		DocType::Knowledge,
		source_ref,
		content_hash,
	);
	let chunk_id = docs::doc_chunk_id_for(doc_id, 0);
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
	let capture = docs::build_source_capture_summary(SourceCaptureSummaryInput {
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
		docs::source_span_id(content_hash, 0, 42, "captured")
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
	let doc_id = docs::source_record_id_for(
		TENANT_ID,
		PROJECT_ID,
		"owner",
		"project_shared",
		DocType::Knowledge,
		source_ref_map,
		"stored-hash",
	);
	let capture = docs::build_source_capture_summary(SourceCaptureSummaryInput {
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
	let normalized = docs::normalize_source_ref_for_capture(source_ref, &capture)
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
fn docs_l0_pointer_carries_hashes_and_position_locator() {
	let now = OffsetDateTime::parse("2026-02-25T12:00:00Z", &Rfc3339)
		.expect("Expected test timestamp to parse.");
	let row = DocSearchRow {
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
	let pointer = docs::build_docs_l0_pointer(&row, row.chunk_id);

	assert_eq!(pointer.schema, "source_ref/v1");
	assert_eq!(pointer.resolver, "elf_doc_ext/v1");
	assert_eq!(pointer.hashes.content_hash, "doc-hash");
	assert_eq!(pointer.hashes.chunk_hash, "chunk-hash");
	assert_eq!(pointer.reference.source_record_id, row.doc_id);
	assert_eq!(pointer.reference.source_span_id, pointer.locator.span_id);
	assert_eq!(pointer.locator.position.start, 12);
	assert_eq!(pointer.locator.position.end, 64);
	assert_eq!(pointer.locator.span_id, docs::source_span_id("doc-hash", 12, 64, "captured"));
	assert_eq!(pointer.state.content_hash, pointer.hashes.content_hash);
	assert_eq!(pointer.state.chunk_hash, pointer.hashes.chunk_hash);
}
