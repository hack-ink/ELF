#[derive(Debug, sqlx::FromRow)]
pub struct MemoryNote {
    pub note_id: uuid::Uuid,
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
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
    pub expires_at: Option<time::OffsetDateTime>,
    pub embedding_version: String,
    pub source_ref: serde_json::Value,
    pub hit_count: i64,
    pub last_hit_at: Option<time::OffsetDateTime>,
}

#[derive(Debug)]
pub struct NoteEmbedding {
    pub note_id: uuid::Uuid,
    pub embedding_version: String,
    pub embedding_dim: i32,
    pub vec: Vec<f32>,
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug)]
pub struct IndexingOutboxEntry {
    pub outbox_id: uuid::Uuid,
    pub note_id: uuid::Uuid,
    pub op: String,
    pub embedding_version: String,
    pub status: String,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub available_at: time::OffsetDateTime,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}
