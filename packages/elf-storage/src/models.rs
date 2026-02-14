use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct MemoryNote {
	pub note_id: Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: String,
	pub r#type: String,
	pub key: Option<String>,
	pub text: String,
	pub importance: f32,
	pub confidence: f32,
	pub status: String,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub expires_at: Option<OffsetDateTime>,
	pub embedding_version: String,
	pub source_ref: Value,
	pub hit_count: i64,
	pub last_hit_at: Option<OffsetDateTime>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct MemoryNoteChunk {
	pub chunk_id: Uuid,
	pub note_id: Uuid,
	pub chunk_index: i32,
	pub start_offset: i32,
	pub end_offset: i32,
	pub text: String,
	pub embedding_version: String,
	pub created_at: OffsetDateTime,
}

#[derive(Debug, sqlx::FromRow)]
pub struct NoteChunkEmbedding {
	pub chunk_id: Uuid,
	pub embedding_version: String,
	pub embedding_dim: i32,
	pub vec: Vec<f32>,
	pub created_at: OffsetDateTime,
}

#[derive(Debug)]
pub struct NoteEmbedding {
	pub note_id: Uuid,
	pub embedding_version: String,
	pub embedding_dim: i32,
	pub vec: Vec<f32>,
	pub created_at: OffsetDateTime,
}

#[derive(Debug, sqlx::FromRow)]
pub struct IndexingOutboxEntry {
	pub outbox_id: Uuid,
	pub note_id: Uuid,
	pub op: String,
	pub embedding_version: String,
	pub status: String,
	pub attempts: i32,
	pub last_error: Option<String>,
	pub available_at: OffsetDateTime,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
}
