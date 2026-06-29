use std::collections::HashMap;

use uuid::Uuid;

use crate::search::{
	DiversityDecision, ScoredChunk,
	ranking::{policy::ResolvedDiversityPolicy, retrieval},
};

use super::similarity::nearest_selected_similarity;

#[derive(Clone, Copy)]
struct DiversityPick {
	remaining_pos: usize,
	mmr_score: f32,
	nearest_note_id: Option<Uuid>,
	similarity: Option<f32>,
	missing_embedding: bool,
	retrieval_rank: u32,
}

impl DiversityPick {
	fn better_than(self, other: &Self) -> bool {
		self.mmr_score > other.mmr_score
			|| (self.mmr_score == other.mmr_score && self.retrieval_rank < other.retrieval_rank)
	}
}

pub fn select_diverse_results(
	candidates: Vec<ScoredChunk>,
	top_k: u32,
	policy: &ResolvedDiversityPolicy,
	note_vectors: &HashMap<Uuid, Vec<f32>>,
) -> (Vec<ScoredChunk>, HashMap<Uuid, DiversityDecision>) {
	if candidates.is_empty() || top_k == 0 {
		return (Vec::new(), HashMap::new());
	}
	if !policy.enabled {
		return select_diverse_results_disabled(candidates, top_k, note_vectors);
	}

	select_diverse_results_enabled(candidates, top_k, policy, note_vectors)
}

fn select_diverse_results_disabled(
	candidates: Vec<ScoredChunk>,
	top_k: u32,
	note_vectors: &HashMap<Uuid, Vec<f32>>,
) -> (Vec<ScoredChunk>, HashMap<Uuid, DiversityDecision>) {
	let mut decisions = HashMap::new();
	let mut selected = Vec::new();

	for (idx, candidate) in candidates.into_iter().enumerate() {
		let selected_rank = (idx < top_k as usize).then_some(idx as u32 + 1);
		let is_selected = selected_rank.is_some();
		let note_id = candidate.item.note.note_id;
		let missing_embedding = !note_vectors.contains_key(&note_id);

		decisions.insert(
			note_id,
			DiversityDecision {
				selected: is_selected,
				selected_rank,
				selected_reason: if is_selected {
					"disabled_passthrough".to_string()
				} else {
					"disabled_truncate".to_string()
				},
				skipped_reason: if is_selected {
					None
				} else {
					Some("disabled_truncate".to_string())
				},
				nearest_selected_note_id: None,
				similarity: None,
				mmr_score: None,
				missing_embedding,
			},
		);

		if is_selected {
			selected.push(candidate);
		}
	}

	(selected, decisions)
}

fn select_diverse_results_enabled(
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
			nearest_selected_similarity(note_id, &candidates, &selected_indices, note_vectors);
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
			nearest_selected_similarity(note_id, candidates, selected_indices, note_vectors);
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
