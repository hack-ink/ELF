use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, FromRow)]
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

#[derive(Debug, FromRow)]
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

#[derive(Debug, FromRow)]
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

#[derive(Debug, FromRow)]
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

#[derive(Debug, FromRow)]
pub struct TraceOutboxJob {
	pub outbox_id: Uuid,
	pub trace_id: Uuid,
	pub payload: Value,
	pub attempts: i32,
}

#[derive(Debug, FromRow)]
pub struct GraphEntity {
	pub entity_id: Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub canonical: String,
	pub canonical_norm: String,
	pub kind: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
pub struct GraphEntityAlias {
	pub alias_id: Uuid,
	pub entity_id: Uuid,
	pub alias: String,
	pub alias_norm: String,
	pub created_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
pub struct GraphFact {
	pub fact_id: Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: String,
	pub subject_entity_id: Uuid,
	pub predicate: String,
	pub predicate_id: Option<Uuid>,
	pub object_entity_id: Option<Uuid>,
	pub object_value: Option<String>,
	pub valid_from: OffsetDateTime,
	pub valid_to: Option<OffsetDateTime>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
pub struct GraphFactEvidence {
	pub evidence_id: Uuid,
	pub fact_id: Uuid,
	pub note_id: Uuid,
	pub created_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
pub struct GraphPredicate {
	pub predicate_id: Uuid,
	pub scope_key: String,
	pub tenant_id: Option<String>,
	pub project_id: Option<String>,
	pub canonical: String,
	pub canonical_norm: String,
	pub cardinality: String,
	pub status: String,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
pub struct GraphPredicateAlias {
	pub alias_id: Uuid,
	pub predicate_id: Uuid,
	pub scope_key: String,
	pub alias: String,
	pub alias_norm: String,
	pub created_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
pub struct GraphFactSupersession {
	pub supersession_id: Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub from_fact_id: Uuid,
	pub to_fact_id: Uuid,
	pub note_id: Uuid,
	pub effective_at: OffsetDateTime,
	pub created_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
pub struct DocDocument {
	pub doc_id: Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: String,
	pub doc_type: String,
	pub status: String,
	pub title: Option<String>,
	pub source_ref: Value,
	pub content: String,
	pub content_bytes: i32,
	pub content_hash: String,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
pub struct DocChunk {
	pub chunk_id: Uuid,
	pub doc_id: Uuid,
	pub chunk_index: i32,
	pub start_offset: i32,
	pub end_offset: i32,
	pub chunk_text: String,
	pub chunk_hash: String,
	pub created_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
pub struct DocChunkEmbedding {
	pub chunk_id: Uuid,
	pub embedding_version: String,
	pub embedding_dim: i32,
	pub vec: Vec<f32>,
	pub created_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
pub struct DocIndexingOutboxEntry {
	pub outbox_id: Uuid,
	pub doc_id: Uuid,
	pub chunk_id: Uuid,
	pub op: String,
	pub embedding_version: String,
	pub status: String,
	pub attempts: i32,
	pub last_error: Option<String>,
	pub available_at: OffsetDateTime,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
}
