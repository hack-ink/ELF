use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(super) struct TraceRow {
	pub(super) trace_id: Uuid,
	pub(super) tenant_id: String,
	pub(super) project_id: String,
	pub(super) agent_id: String,
	pub(super) read_profile: String,
	pub(super) query: String,
	pub(super) expansion_mode: String,
	pub(super) expanded_queries: Value,
	pub(super) allowed_scopes: Value,
	pub(super) candidate_count: i32,
	pub(super) top_k: i32,
	pub(super) config_snapshot: Value,
	pub(super) trace_version: i32,
	pub(super) created_at: OffsetDateTime,
	pub(super) expires_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
pub(super) struct CandidateRow {
	pub(super) candidate_id: Uuid,
	pub(super) trace_id: Uuid,
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

#[derive(Debug, FromRow)]
pub(super) struct ItemRow {
	pub(super) item_id: Uuid,
	pub(super) trace_id: Uuid,
	pub(super) note_id: Uuid,
	pub(super) chunk_id: Option<Uuid>,
	pub(super) rank: i32,
	pub(super) final_score: f32,
	pub(super) explain: Value,
}

#[derive(Debug, FromRow)]
pub(super) struct StageRow {
	pub(super) stage_id: Uuid,
	pub(super) trace_id: Uuid,
	pub(super) stage_order: i32,
	pub(super) stage_name: String,
	pub(super) stage_payload: Value,
	pub(super) created_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
pub(super) struct StageItemRow {
	pub(super) id: Uuid,
	pub(super) stage_id: Uuid,
	pub(super) item_id: Option<Uuid>,
	pub(super) note_id: Option<Uuid>,
	pub(super) chunk_id: Option<Uuid>,
	pub(super) metrics: Value,
}
