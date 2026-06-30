use uuid::Uuid;

use crate::app::trace_compare::types::TraceCompareCandidateRow;
use elf_service::search::TraceReplayCandidate;

pub(in crate::app::trace_compare) fn decode_trace_replay_candidates(
	rows: Vec<TraceCompareCandidateRow>,
) -> Vec<TraceReplayCandidate> {
	rows.into_iter()
		.map(|row| {
			let decoded =
				serde_json::from_value::<TraceReplayCandidate>(row.candidate_snapshot.clone())
					.ok()
					.filter(|value| value.note_id != Uuid::nil() && value.chunk_id != Uuid::nil());

			decoded.unwrap_or_else(|| TraceReplayCandidate {
				note_id: row.note_id,
				chunk_id: row.chunk_id,
				chunk_index: row.chunk_index,
				snippet: row.snippet,
				retrieval_rank: u32::try_from(row.retrieval_rank).unwrap_or(0),
				retrieval_score: None,
				rerank_score: row.rerank_score,
				note_scope: row.note_scope,
				note_importance: row.note_importance,
				note_updated_at: row.note_updated_at,
				note_hit_count: row.note_hit_count,
				note_last_hit_at: row.note_last_hit_at,
				diversity_selected: None,
				diversity_selected_rank: None,
				diversity_selected_reason: None,
				diversity_skipped_reason: None,
				diversity_nearest_selected_note_id: None,
				diversity_similarity: None,
				diversity_mmr_score: None,
				diversity_missing_embedding: None,
			})
		})
		.collect()
}
