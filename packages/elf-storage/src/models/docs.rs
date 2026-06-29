use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

/// Persisted document row.
#[derive(Debug, FromRow)]
pub struct DocDocument {
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
	/// Document type discriminator.
	pub doc_type: String,
	/// Lifecycle status for the document.
	pub status: String,
	/// Optional document title.
	pub title: Option<String>,
	/// Structured source reference metadata.
	pub source_ref: Value,
	/// Full document content.
	pub content: String,
	/// Byte length of the document content.
	pub content_bytes: i32,
	/// Content hash for deduplication and change detection.
	pub content_hash: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Persisted chunk row for one document.
#[derive(Debug, FromRow)]
pub struct DocChunk {
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// Parent document identifier.
	pub doc_id: Uuid,
	/// Zero-based chunk position within the document.
	pub chunk_index: i32,
	/// Inclusive start byte offset within the original document content.
	pub start_offset: i32,
	/// Exclusive end byte offset within the original document content.
	pub end_offset: i32,
	/// Chunk text.
	pub chunk_text: String,
	/// Chunk content hash.
	pub chunk_hash: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted embedding row for one document chunk.
#[derive(Debug, FromRow)]
pub struct DocChunkEmbedding {
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// Embedding version associated with the vector.
	pub embedding_version: String,
	/// Embedding dimensionality.
	pub embedding_dim: i32,
	/// Embedding vector payload.
	pub vec: Vec<f32>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted document-indexing outbox row.
#[derive(Debug, FromRow)]
pub struct DocIndexingOutboxEntry {
	/// Outbox identifier.
	pub outbox_id: Uuid,
	/// Document identifier queued for indexing.
	pub doc_id: Uuid,
	/// Chunk identifier queued for indexing.
	pub chunk_id: Uuid,
	/// Requested indexing operation.
	pub op: String,
	/// Embedding version the worker should use.
	pub embedding_version: String,
	/// Current outbox status.
	pub status: String,
	/// Number of attempts already made.
	pub attempts: i32,
	/// Most recent failure text, if any.
	pub last_error: Option<String>,
	/// Earliest time the job may be claimed again.
	pub available_at: OffsetDateTime,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}
