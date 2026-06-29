use super::*;

pub(super) type ProjectDocRefFields = (String, Option<String>, Option<String>, Option<String>);

pub(super) const POLL_INTERVAL_MS: i64 = 500;
pub(super) const CLAIM_LEASE_SECONDS: i64 = 30;
pub(super) const BASE_BACKOFF_MS: i64 = 500;
pub(super) const MAX_BACKOFF_MS: i64 = 30_000;
pub(super) const TRACE_CLEANUP_INTERVAL_SECONDS: i64 = 900;
pub(super) const TRACE_OUTBOX_LEASE_SECONDS: i64 = 30;
pub(super) const CONSOLIDATION_JOB_LEASE_SECONDS: i64 = 30;
pub(super) const MAX_OUTBOX_ERROR_CHARS: usize = 1_024;

/// Shared runtime state used by the worker loop.
pub struct WorkerState {
	/// Postgres storage handle.
	pub db: Db,
	/// Note-index Qdrant collection handle.
	pub qdrant: QdrantStore,
	/// Document-index Qdrant collection handle.
	pub docs_qdrant: QdrantStore,
	/// Embedding provider configuration.
	pub embedding: EmbeddingProviderConfig,
	/// Chunking configuration for notes and docs.
	pub chunking: ChunkingConfig,
	/// Tokenizer used for chunking operations.
	pub tokenizer: Tokenizer,
}

#[derive(Debug, Deserialize)]
pub(super) struct TracePayload {
	pub(super) trace: TraceRecord,
	pub(super) items: Vec<TraceItemRecord>,
	#[serde(default)]
	pub(super) candidates: Vec<TraceCandidateRecord>,
	#[serde(default)]
	pub(super) stages: Vec<TraceTrajectoryStageRecord>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TraceRecord {
	pub(super) trace_id: Uuid,
	pub(super) tenant_id: String,
	pub(super) project_id: String,
	pub(super) agent_id: String,
	pub(super) read_profile: String,
	pub(super) query: String,
	pub(super) expansion_mode: String,
	pub(super) expanded_queries: Vec<String>,
	pub(super) allowed_scopes: Vec<String>,
	pub(super) candidate_count: u32,
	pub(super) top_k: u32,
	pub(super) config_snapshot: Value,
	pub(super) trace_version: i32,
	pub(super) created_at: OffsetDateTime,
	pub(super) expires_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub(super) struct TraceItemRecord {
	pub(super) item_id: Uuid,
	pub(super) note_id: Uuid,
	pub(super) chunk_id: Option<Uuid>,
	pub(super) rank: u32,
	pub(super) final_score: f32,
	pub(super) explain: Value,
}

#[derive(Debug, Deserialize)]
pub(super) struct TraceCandidateRecord {
	pub(super) candidate_id: Uuid,
	pub(super) note_id: Uuid,
	pub(super) chunk_id: Uuid,
	#[serde(default)]
	pub(super) chunk_index: i32,
	#[serde(default)]
	pub(super) snippet: String,
	#[serde(default)]
	pub(super) candidate_snapshot: Value,
	pub(super) retrieval_rank: u32,
	pub(super) rerank_score: f32,
	pub(super) note_scope: String,
	pub(super) note_importance: f32,
	pub(super) note_updated_at: OffsetDateTime,
	#[serde(default)]
	pub(super) note_hit_count: i64,
	pub(super) note_last_hit_at: Option<OffsetDateTime>,
	pub(super) created_at: OffsetDateTime,
	pub(super) expires_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub(super) struct TraceTrajectoryStageRecord {
	pub(super) stage_id: Uuid,
	pub(super) stage_order: u32,
	pub(super) stage_name: String,
	pub(super) stage_payload: Value,
	pub(super) created_at: OffsetDateTime,
	#[serde(default)]
	pub(super) items: Vec<TraceTrajectoryStageItemRecord>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TraceTrajectoryStageItemRecord {
	pub(super) id: Uuid,
	pub(super) item_id: Option<Uuid>,
	pub(super) note_id: Option<Uuid>,
	pub(super) chunk_id: Option<Uuid>,
	pub(super) metrics: Value,
}

pub(super) struct TraceItemInsert {
	pub(super) item_id: Uuid,
	pub(super) note_id: Uuid,
	pub(super) chunk_id: Option<Uuid>,
	pub(super) rank: i32,
	pub(super) final_score: f32,
	pub(super) explain: Value,
}

pub(super) struct TraceCandidateInsert {
	pub(super) candidate_id: Uuid,
	pub(super) note_id: Uuid,
	pub(super) chunk_id: Uuid,
	pub(super) chunk_index: i32,
	pub(super) snippet: String,
	pub(super) candidate_snapshot: Value,
	pub(super) retrieval_rank: i32,
	pub(super) rerank_score: f32,
	pub(super) note_scope: String,
	pub(super) note_importance: f32,
	pub(super) note_updated_at: OffsetDateTime,
	pub(super) note_hit_count: i64,
	pub(super) note_last_hit_at: Option<OffsetDateTime>,
	pub(super) created_at: OffsetDateTime,
	pub(super) expires_at: OffsetDateTime,
}

pub(super) struct TraceStageInsert {
	pub(super) stage_id: Uuid,
	pub(super) stage_order: i32,
	pub(super) stage_name: String,
	pub(super) stage_payload: Value,
	pub(super) created_at: OffsetDateTime,
}

pub(super) struct TraceStageItemInsert {
	pub(super) id: Uuid,
	pub(super) stage_id: Uuid,
	pub(super) item_id: Option<Uuid>,
	pub(super) note_id: Option<Uuid>,
	pub(super) chunk_id: Option<Uuid>,
	pub(super) metrics: Value,
}

pub(super) struct ChunkRecord {
	pub(super) chunk_id: Uuid,
	pub(super) chunk_index: i32,
	pub(super) start_offset: i32,
	pub(super) end_offset: i32,
	pub(super) text: String,
}

#[derive(Debug, FromRow)]
pub(super) struct NoteFieldRow {
	pub(super) field_id: Uuid,
	pub(super) text: String,
}

#[derive(Debug, FromRow)]
pub(super) struct DocChunkIndexRow {
	pub(super) doc_id: Uuid,
	pub(super) tenant_id: String,
	pub(super) project_id: String,
	pub(super) agent_id: String,
	pub(super) scope: String,
	pub(super) doc_type: String,
	pub(super) status: String,
	pub(super) created_at: OffsetDateTime,
	pub(super) updated_at: OffsetDateTime,
	pub(super) content_hash: String,
	pub(super) source_ref: Value,
	pub(super) chunk_id: Uuid,
	pub(super) chunk_index: i32,
	pub(super) start_offset: i32,
	pub(super) end_offset: i32,
	pub(super) chunk_text: String,
	pub(super) chunk_hash: String,
}
