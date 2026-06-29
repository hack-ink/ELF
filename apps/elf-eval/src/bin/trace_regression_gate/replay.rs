use std::collections::HashSet;

use uuid::Uuid;

use crate::rows::CandidateRow;

pub(super) fn decode_trace_replay_candidates(
	rows: Vec<CandidateRow>,
) -> Vec<elf_service::search::TraceReplayCandidate> {
	rows.into_iter()
		.map(|row| {
			let decoded = serde_json::from_value::<elf_service::search::TraceReplayCandidate>(
				row.candidate_snapshot.clone(),
			)
			.ok()
			.filter(|value| value.note_id != Uuid::nil() && value.chunk_id != Uuid::nil());

			decoded.unwrap_or_else(|| elf_service::search::TraceReplayCandidate {
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

pub(super) fn churn_against_baseline_at_k(
	baseline: &[Uuid],
	other: &[Uuid],
	k: usize,
) -> (f64, f64) {
	let k = k.max(1);
	let mut positional_diff = 0_usize;

	for idx in 0..k {
		let a = baseline.get(idx);
		let b = other.get(idx);

		if a != b {
			positional_diff += 1;
		}
	}

	let positional_churn = positional_diff as f64 / k as f64;
	let base_set: HashSet<Uuid> = baseline.iter().take(k).copied().collect();
	let other_set: HashSet<Uuid> = other.iter().take(k).copied().collect();
	let overlap = base_set.intersection(&other_set).count();
	let set_churn = 1.0 - (overlap as f64 / k as f64);

	(positional_churn, set_churn)
}

pub(super) fn retrieval_top_rank_retention(
	candidates: &[elf_service::search::TraceReplayCandidate],
	note_ids: &[Uuid],
	max_retrieval_rank: u32,
) -> (usize, usize, f64) {
	let mut top_notes = HashSet::new();

	for candidate in candidates {
		if candidate.retrieval_rank == 0 || candidate.retrieval_rank > max_retrieval_rank {
			continue;
		}

		top_notes.insert(candidate.note_id);
	}

	let total = top_notes.len();

	if total == 0 {
		return (0, 0, 0.0);
	}

	let out_set: HashSet<Uuid> = note_ids.iter().copied().collect();
	let retained = top_notes.intersection(&out_set).count();
	let retention = retained as f64 / total as f64;

	(total, retained, retention)
}
