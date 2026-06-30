use std::collections::HashMap;

use uuid::Uuid;

use crate::search::{
	DiversityDecision, ScoredChunk,
	ranking::{
		diversity::{selection::pick::DiversityPick, similarity},
		policy::ResolvedDiversityPolicy,
		retrieval,
	},
};

pub(super) fn select_diverse_results_enabled(
	candidates: Vec<ScoredChunk>,
	top_k: u32,
	policy: &ResolvedDiversityPolicy,
	note_vectors: &HashMap<Uuid, Vec<f32>>,
) -> (Vec<ScoredChunk>, HashMap<Uuid, DiversityDecision>) {
	let total = u32::try_from(candidates.len()).unwrap_or(1).max(1);
	let relevance_by_idx: Vec<f32> =
		(0..candidates.len()).map(|idx| retrieval::rank_normalize(idx as u32 + 1, total)).collect();
	let mut remaining_indices: Vec<usize> = (0..candidates.len()).collect();
	let mut selected_indices: Vec<usize> = Vec::new();
	let mut decisions: HashMap<Uuid, DiversityDecision> = HashMap::new();
	let first_idx = remaining_indices.remove(0);
	let first_note_id = candidates[first_idx].item.note.note_id;
	let first_missing_embedding = !note_vectors.contains_key(&first_note_id);

	selected_indices.push(first_idx);
	decisions.insert(
		first_note_id,
		DiversityDecision {
			selected: true,
			selected_rank: Some(1),
			selected_reason: "top_relevance".to_string(),
			skipped_reason: None,
			nearest_selected_note_id: None,
			similarity: None,
			mmr_score: Some(relevance_by_idx[first_idx]),
			missing_embedding: first_missing_embedding,
		},
	);

	while selected_indices.len() < top_k as usize && !remaining_indices.is_empty() {
		let Some((selected_pick, selected_reason)) = pick_next_candidate(
			&remaining_indices,
			&candidates,
			&selected_indices,
			note_vectors,
			&relevance_by_idx,
			policy,
		) else {
			break;
		};
		let picked_idx = remaining_indices.remove(selected_pick.remaining_pos);

		selected_indices.push(picked_idx);

		let selected_note_id = candidates[picked_idx].item.note.note_id;

		decisions.insert(
			selected_note_id,
			DiversityDecision {
				selected: true,
				selected_rank: Some(selected_indices.len() as u32),
				selected_reason: selected_reason.to_string(),
				skipped_reason: None,
				nearest_selected_note_id: selected_pick.nearest_note_id,
				similarity: selected_pick.similarity,
				mmr_score: Some(selected_pick.mmr_score),
				missing_embedding: selected_pick.missing_embedding,
			},
		);
	}

	for candidate_idx in remaining_indices {
		let note_id = candidates[candidate_idx].item.note.note_id;
		let (similarity, nearest_note_id, missing_embedding) =
			similarity::nearest_selected_similarity(
				note_id,
				&candidates,
				&selected_indices,
				note_vectors,
			);
		let skipped_reason =
			if similarity.map(|value| value > policy.sim_threshold).unwrap_or(false) {
				"similarity_threshold"
			} else {
				"lower_mmr"
			};
		let redundancy = similarity.unwrap_or(0.0);
		let mmr_score = policy.mmr_lambda * relevance_by_idx[candidate_idx]
			- (1.0 - policy.mmr_lambda) * redundancy;

		decisions.insert(
			note_id,
			DiversityDecision {
				selected: false,
				selected_rank: None,
				selected_reason: "not_selected".to_string(),
				skipped_reason: Some(skipped_reason.to_string()),
				nearest_selected_note_id: nearest_note_id,
				similarity,
				mmr_score: Some(mmr_score),
				missing_embedding,
			},
		);
	}

	let selected = selected_indices.into_iter().map(|idx| candidates[idx].clone()).collect();

	(selected, decisions)
}

fn pick_next_candidate(
	remaining_indices: &[usize],
	candidates: &[ScoredChunk],
	selected_indices: &[usize],
	note_vectors: &HashMap<Uuid, Vec<f32>>,
	relevance_by_idx: &[f32],
	policy: &ResolvedDiversityPolicy,
) -> Option<(DiversityPick, &'static str)> {
	let mut best_non_filtered: Option<DiversityPick> = None;
	let mut best_filtered: Option<DiversityPick> = None;
	let mut best_any: Option<DiversityPick> = None;
	let mut filtered_count = 0_u32;

	for (remaining_pos, candidate_idx) in remaining_indices.iter().copied().enumerate() {
		let note_id = candidates[candidate_idx].item.note.note_id;
		let (similarity, nearest_note_id, missing_embedding) =
			similarity::nearest_selected_similarity(
				note_id,
				candidates,
				selected_indices,
				note_vectors,
			);
		let redundancy = similarity.unwrap_or(0.0);
		let mmr_score = policy.mmr_lambda * relevance_by_idx[candidate_idx]
			- (1.0 - policy.mmr_lambda) * redundancy;
		let high_similarity = similarity.map(|value| value > policy.sim_threshold).unwrap_or(false);

		if high_similarity {
			filtered_count += 1;
		}

		let candidate_pick = DiversityPick {
			remaining_pos,
			mmr_score,
			nearest_note_id,
			similarity,
			missing_embedding,
			retrieval_rank: candidates[candidate_idx].item.retrieval_rank,
		};

		if best_any.as_ref().map(|current| candidate_pick.better_than(current)).unwrap_or(true) {
			best_any = Some(candidate_pick);
		}
		if high_similarity {
			if best_filtered
				.as_ref()
				.map(|current| candidate_pick.better_than(current))
				.unwrap_or(true)
			{
				best_filtered = Some(candidate_pick);
			}

			continue;
		}
		if best_non_filtered
			.as_ref()
			.map(|current| candidate_pick.better_than(current))
			.unwrap_or(true)
		{
			best_non_filtered = Some(candidate_pick);
		}
	}

	if let Some(best) = best_non_filtered {
		return Some((best, "mmr"));
	}

	if filtered_count >= policy.max_skips {
		return best_any.map(|best| (best, "max_skips_backfill"));
	}

	best_filtered.map(|best| (best, "threshold_backfill"))
}
