use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use elf_domain::writegate::{WritePolicy, WritePolicyAudit};

/// Request payload for document ingestion.
#[derive(Clone, Debug, Deserialize)]
pub struct DocsPutRequest {
	/// Tenant that owns the document.
	pub tenant_id: String,
	/// Project that owns the document.
	pub project_id: String,
	/// Agent ingesting the document.
	pub agent_id: String,
	/// Scope to assign to the document.
	pub scope: String,
	/// Optional raw document-type string.
	pub doc_type: Option<String>,
	/// Optional display title for the document.
	pub title: Option<String>,
	/// Optional write policy applied before persistence.
	pub write_policy: Option<WritePolicy>,
	#[serde(default)]
	/// Structured provenance metadata for the document.
	pub source_ref: Value,
	/// Full document body to store and chunk.
	pub content: String,
}

/// Response payload for document ingestion.
#[derive(Clone, Debug, Serialize)]
pub struct DocsPutResponse {
	/// Identifier of the stored document.
	pub doc_id: Uuid,
	/// Normalized Source Library capture metadata for the stored document.
	pub source_capture: DocsSourceCaptureSummary,
	/// Number of persisted chunks generated from the content.
	pub chunk_count: u32,
	/// Byte length of the stored content.
	pub content_bytes: u32,
	/// Whole-document BLAKE3 hash.
	pub content_hash: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Write-policy audit emitted for the stored document, when applicable.
	pub write_policy_audit: Option<WritePolicyAudit>,
}

/// Normalized Source Library capture metadata returned by `docs_put`.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSourceCaptureSummary {
	/// Schema identifier for this capture summary.
	pub schema: String,
	/// Stable source record identifier. This is also the stored `doc_id`.
	pub source_record_id: Uuid,
	/// Source-record lifecycle and freshness state at capture time.
	pub lifecycle: DocsSourceLifecycle,
	/// Canonical source origin used for operator inspection and deduplication.
	pub origin: String,
	/// RFC3339 timestamp when ELF captured the source.
	pub captured_at: String,
	/// Whole-document BLAKE3 hash for the persisted content.
	pub content_hash: String,
	/// Visibility scope assigned to the source record.
	pub visibility_scope: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional display title associated with the source record.
	pub title: Option<String>,
	/// Normalized source type, derived from `source_kind` when present.
	pub source_type: String,
	/// Stable span references for persisted source chunks.
	pub source_spans: Vec<DocsSourceSpanRef>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	/// Typed audit records for redacted or excluded source spans.
	pub policy_spans: Vec<DocsSourceSpanRef>,
}

/// Source Library lifecycle metadata persisted with normalized source refs.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSourceLifecycle {
	/// Schema identifier for this lifecycle object.
	pub schema: String,
	/// Authoritative Source Library row status.
	pub status: String,
	/// Freshness label used by recall and audit surfaces.
	pub freshness: String,
	/// Actor that created the current lifecycle transition.
	pub actor_agent_id: String,
	/// Transition timestamp.
	pub ts: String,
	/// Machine-readable lifecycle reason.
	pub reason_code: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Timestamp when the source was tombstoned or deleted.
	pub deleted_at: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Review or delete metadata that proves the tombstone.
	pub tombstone_ref: Option<Value>,
}

/// Stable reference to one captured or policy-affected source span.
#[derive(Clone, Debug, Serialize)]
pub struct DocsSourceSpanRef {
	/// Schema identifier for this span reference.
	pub schema: String,
	/// Stable span identifier derived from content hash and byte offsets.
	pub span_id: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Chunk identifier when this span is backed by a persisted chunk.
	pub chunk_id: Option<Uuid>,
	/// Span lifecycle status such as `captured`, `excluded`, or `redacted`.
	pub status: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Typed reason code for non-captured spans.
	pub reason_code: Option<String>,
	/// Inclusive start byte offset in the relevant content hash.
	pub start_offset: usize,
	/// Exclusive end byte offset in the relevant content hash.
	pub end_offset: usize,
	/// Whole-content hash that makes the offsets replayable.
	pub content_hash: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Chunk hash when this span is backed by a persisted chunk.
	pub chunk_hash: Option<String>,
}
