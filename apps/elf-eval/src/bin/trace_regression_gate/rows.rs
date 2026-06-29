use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(super) struct TraceRow {
	pub(super) trace_id: Uuid,
	pub(super) query: String,
	pub(super) candidate_count: i32,
	pub(super) top_k: i32,
	pub(super) created_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
pub(super) struct TraceItemRow {
	pub(super) note_id: Uuid,
}

#[derive(Debug, FromRow)]
pub(super) struct CandidateRow {
	pub(super) candidate_snapshot: Value,
	pub(super) note_id: Uuid,
	pub(super) chunk_id: Uuid,
	pub(super) chunk_index: i32,
	pub(super) snippet: String,
	pub(super) retrieval_rank: i32,
	pub(super) rerank_score: f32,
	pub(super) note_scope: String,
	pub(super) note_importance: f32,
	pub(super) note_updated_at: OffsetDateTime,
	pub(super) note_hit_count: i64,
	pub(super) note_last_hit_at: Option<OffsetDateTime>,
}
