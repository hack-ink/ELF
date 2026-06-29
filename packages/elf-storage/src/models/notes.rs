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
