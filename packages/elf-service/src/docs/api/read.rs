use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::NoteOp;

/// Request payload for document metadata lookup.
#[derive(Clone, Debug, Deserialize)]
pub struct DocsGetRequest {
	/// Tenant that owns the document.
	pub tenant_id: String,
	/// Project that owns the document.
	pub project_id: String,
	/// Agent requesting the read.
	pub agent_id: String,
	/// Read profile that determines visible scopes.
	pub read_profile: String,
	/// Identifier of the document to fetch.
	pub doc_id: Uuid,
}

/// Response payload for document metadata lookup.
#[derive(Clone, Debug, Serialize)]
pub struct DocsGetResponse {
	/// Document identifier.
	pub doc_id: Uuid,
	/// Tenant that owns the document.
	pub tenant_id: String,
	/// Project that owns the document.
	pub project_id: String,
	/// Agent that ingested the document.
	pub agent_id: String,
	/// Scope key for the document.
	pub scope: String,
	/// Stored document type.
	pub doc_type: String,
	/// Lifecycle status for the document.
	pub status: String,
	/// Optional document title.
	pub title: Option<String>,
	/// Structured provenance metadata.
	pub source_ref: Value,
	/// Byte length of the stored content.
	pub content_bytes: u32,
	/// Whole-document BLAKE3 hash.
	pub content_hash: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Request payload for Source Library document deletion.
#[derive(Clone, Debug, Deserialize)]
pub struct DocsDeleteRequest {
	/// Tenant that owns the document.
	pub tenant_id: String,
	/// Project that owns the document.
	pub project_id: String,
	/// Agent requesting the deletion.
	pub agent_id: String,
	/// Identifier of the document to delete.
	pub doc_id: Uuid,
}

/// Response payload for Source Library document deletion.
#[derive(Clone, Debug, Serialize)]
pub struct DocsDeleteResponse {
	/// Identifier of the affected document.
	pub doc_id: Uuid,
	/// Operation that was applied.
	pub op: NoteOp,
	/// Number of persisted chunks queued for derived-index deletion.
	pub chunk_delete_count: u32,
}
