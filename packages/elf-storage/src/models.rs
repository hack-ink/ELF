//! Database row models shared across storage modules.

use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

/// Persisted memory note row.
#[derive(Debug, FromRow)]
pub struct MemoryNote {
	/// Note identifier.
	pub note_id: Uuid,
	/// Tenant that owns the note.
	pub tenant_id: String,
	/// Project that owns the note.
	pub project_id: String,
	/// Agent that wrote the note.
	pub agent_id: String,
	/// Scope key for the note.
	pub scope: String,
	/// Note type discriminator.
	pub r#type: String,
	/// Optional application-defined key for deduplication or lookup.
	pub key: Option<String>,
	/// Note body text.
	pub text: String,
	/// Importance score persisted for ranking.
	pub importance: f32,
	/// Confidence score persisted for ranking.
	pub confidence: f32,
	/// Lifecycle status for the note.
	pub status: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	/// Optional expiry timestamp.
	pub expires_at: Option<OffsetDateTime>,
	/// Embedding version associated with the stored note.
	pub embedding_version: String,
	/// Structured source reference metadata.
	pub source_ref: Value,
	/// Search hit counter.
	pub hit_count: i64,
	/// Timestamp of the most recent search hit.
	pub last_hit_at: Option<OffsetDateTime>,
}

/// Persisted chunk row for one memory note.
#[derive(Debug, FromRow)]
pub struct MemoryNoteChunk {
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// Parent note identifier.
	pub note_id: Uuid,
	/// Zero-based chunk position within the note.
	pub chunk_index: i32,
	/// Inclusive start byte offset within the original note text.
	pub start_offset: i32,
	/// Exclusive end byte offset within the original note text.
	pub end_offset: i32,
	/// Chunk text.
	pub text: String,
	/// Embedding version associated with the chunk.
	pub embedding_version: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted embedding row for one note chunk.
#[derive(Debug, FromRow)]
pub struct NoteChunkEmbedding {
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

/// In-memory embedding payload for a full note.
#[derive(Debug)]
pub struct NoteEmbedding {
	/// Note identifier.
	pub note_id: Uuid,
	/// Embedding version associated with the vector.
	pub embedding_version: String,
	/// Embedding dimensionality.
	pub embedding_dim: i32,
	/// Embedding vector payload.
	pub vec: Vec<f32>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted note-indexing outbox row.
#[derive(Debug, FromRow)]
pub struct IndexingOutboxEntry {
	/// Outbox identifier.
	pub outbox_id: Uuid,
	/// Note identifier queued for indexing.
	pub note_id: Uuid,
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

/// Persisted search-trace outbox job.
#[derive(Debug, FromRow)]
pub struct TraceOutboxJob {
	/// Outbox identifier.
	pub outbox_id: Uuid,
	/// Trace identifier to export.
	pub trace_id: Uuid,
	/// Serialized trace payload.
	pub payload: Value,
	/// Number of attempts already made.
	pub attempts: i32,
}

/// Persisted graph entity row.
#[derive(Debug, FromRow)]
pub struct GraphEntity {
	/// Entity identifier.
	pub entity_id: Uuid,
	/// Tenant that owns the entity.
	pub tenant_id: String,
	/// Project that owns the entity.
	pub project_id: String,
	/// Canonical entity surface.
	pub canonical: String,
	/// Normalized canonical entity surface.
	pub canonical_norm: String,
	/// Optional entity kind.
	pub kind: Option<String>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Persisted alias row for a graph entity.
#[derive(Debug, FromRow)]
pub struct GraphEntityAlias {
	/// Alias identifier.
	pub alias_id: Uuid,
	/// Entity identifier that owns the alias.
	pub entity_id: Uuid,
	/// Alias surface.
	pub alias: String,
	/// Normalized alias surface.
	pub alias_norm: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted graph fact row.
#[derive(Debug, FromRow)]
pub struct GraphFact {
	/// Fact identifier.
	pub fact_id: Uuid,
	/// Tenant that owns the fact.
	pub tenant_id: String,
	/// Project that owns the fact.
	pub project_id: String,
	/// Agent that emitted the fact.
	pub agent_id: String,
	/// Scope key for the fact.
	pub scope: String,
	/// Subject entity identifier.
	pub subject_entity_id: Uuid,
	/// Predicate surface captured with the fact.
	pub predicate: String,
	/// Resolved predicate identifier, when available.
	pub predicate_id: Option<Uuid>,
	/// Object entity identifier for entity-to-entity facts.
	pub object_entity_id: Option<Uuid>,
	/// Scalar object value for entity-to-value facts.
	pub object_value: Option<String>,
	/// Start of the fact validity window.
	pub valid_from: OffsetDateTime,
	/// End of the fact validity window, if superseded.
	pub valid_to: Option<OffsetDateTime>,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Evidence link between one graph fact and one memory note.
#[derive(Debug, FromRow)]
pub struct GraphFactEvidence {
	/// Evidence row identifier.
	pub evidence_id: Uuid,
	/// Fact identifier.
	pub fact_id: Uuid,
	/// Note identifier that supports the fact.
	pub note_id: Uuid,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted graph predicate row.
#[derive(Debug, FromRow)]
pub struct GraphPredicate {
	/// Predicate identifier.
	pub predicate_id: Uuid,
	/// Scope key where the predicate is visible.
	pub scope_key: String,
	/// Tenant scope, when tenant-specific.
	pub tenant_id: Option<String>,
	/// Project scope, when project-specific.
	pub project_id: Option<String>,
	/// Canonical predicate surface.
	pub canonical: String,
	/// Normalized canonical predicate surface.
	pub canonical_norm: String,
	/// Cardinality policy for the predicate.
	pub cardinality: String,
	/// Lifecycle status for the predicate.
	pub status: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Persisted alias row for a graph predicate.
#[derive(Debug, FromRow)]
pub struct GraphPredicateAlias {
	/// Alias identifier.
	pub alias_id: Uuid,
	/// Predicate identifier that owns the alias.
	pub predicate_id: Uuid,
	/// Scope key where the alias resolves.
	pub scope_key: String,
	/// Alias surface.
	pub alias: String,
	/// Normalized alias surface.
	pub alias_norm: String,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Persisted supersession row linking two facts.
#[derive(Debug, FromRow)]
pub struct GraphFactSupersession {
	/// Supersession identifier.
	pub supersession_id: Uuid,
	/// Tenant that owns the supersession record.
	pub tenant_id: String,
	/// Project that owns the supersession record.
	pub project_id: String,
	/// Fact identifier that was superseded.
	pub from_fact_id: Uuid,
	/// Fact identifier that replaced the prior fact.
	pub to_fact_id: Uuid,
	/// Note identifier that justified the supersession.
	pub note_id: Uuid,
	/// Time the supersession took effect.
	pub effective_at: OffsetDateTime,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

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
