use super::*;

/// Context needed to replay ranking against stored candidates.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraceReplayContext {
	/// Trace identifier.
	pub trace_id: Uuid,
	/// Search query text.
	pub query: String,
	/// Candidate count observed during the trace.
	pub candidate_count: u32,
	/// Top-k budget used during the trace.
	pub top_k: u32,
	#[serde(with = "crate::time_serde")]
	/// Trace creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Candidate row used for replaying ranking offline.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraceReplayCandidate {
	/// Note identifier.
	pub note_id: Uuid,
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// Zero-based chunk position.
	pub chunk_index: i32,
	/// Candidate snippet text.
	pub snippet: String,
	/// 1-based retrieval rank.
	pub retrieval_rank: u32,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional merged retrieval score captured before rerank.
	pub retrieval_score: Option<f32>,
	/// Raw rerank-model score.
	pub rerank_score: f32,
	/// Scope key for the note.
	pub note_scope: String,
	/// Note importance score.
	pub note_importance: f32,
	#[serde(with = "crate::time_serde")]
	/// Note last-update timestamp.
	pub note_updated_at: OffsetDateTime,
	/// Note hit counter.
	pub note_hit_count: i64,
	#[serde(with = "crate::time_serde::option")]
	/// Timestamp of the note's most recent hit.
	pub note_last_hit_at: Option<OffsetDateTime>,
	/// Whether the candidate was selected by diversity ranking.
	pub diversity_selected: Option<bool>,
	/// Final selected rank under diversity ranking.
	pub diversity_selected_rank: Option<u32>,
	/// Reason the candidate was selected by diversity ranking.
	pub diversity_selected_reason: Option<String>,
	/// Reason the candidate was skipped by diversity ranking.
	pub diversity_skipped_reason: Option<String>,
	/// Nearest selected note that influenced the diversity decision.
	pub diversity_nearest_selected_note_id: Option<Uuid>,
	/// Similarity to the nearest selected note.
	pub diversity_similarity: Option<f32>,
	/// MMR score used for diversity selection.
	pub diversity_mmr_score: Option<f32>,
	/// Whether the candidate lacked an embedding for diversity scoring.
	pub diversity_missing_embedding: Option<bool>,
}

/// Final replayed ranking item.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraceReplayItem {
	/// Note identifier.
	pub note_id: Uuid,
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// 1-based retrieval rank.
	pub retrieval_rank: u32,
	/// Final replayed score.
	pub final_score: f32,
	/// Recomputed explanation payload.
	pub explain: SearchExplain,
}
