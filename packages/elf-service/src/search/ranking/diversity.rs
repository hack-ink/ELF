use std::{cmp::Ordering, collections::HashMap};

use uuid::Uuid;

use crate::search::{
	ChunkSnippet, DiversityDecision, ScoredChunk, SearchDiversityExplain, TraceCandidateRecord,
	TraceReplayCandidate,
	ranking::{policy::ResolvedDiversityPolicy, retrieval},
};

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

pub fn build_diversity_explain(decision: &DiversityDecision) -> SearchDiversityExplain {
	SearchDiversityExplain {
		enabled: true,
		selected_reason: decision.selected_reason.clone(),
		skipped_reason: decision.skipped_reason.clone(),
		nearest_selected_note_id: decision.nearest_selected_note_id,
		similarity: decision.similarity,
		mmr_score: decision.mmr_score,
		missing_embedding: decision.missing_embedding,
	}
}

pub fn cosine_similarity(lhs: &[f32], rhs: &[f32]) -> Option<f32> {
	if lhs.is_empty() || lhs.len() != rhs.len() {
		return None;
	}

	let mut dot = 0.0_f32;
	let mut lhs_norm = 0.0_f32;
	let mut rhs_norm = 0.0_f32;

	for (l, r) in lhs.iter().zip(rhs.iter()) {
		dot += l * r;
		lhs_norm += l * l;
		rhs_norm += r * r;
	}

	if lhs_norm <= f32::EPSILON || rhs_norm <= f32::EPSILON {
		return None;
	}

	Some((dot / (lhs_norm.sqrt() * rhs_norm.sqrt())).clamp(-1.0, 1.0))
}

pub fn nearest_selected_similarity(
	note_id: Uuid,
	candidates: &[ScoredChunk],
	selected_indices: &[usize],
	note_vectors: &HashMap<Uuid, Vec<f32>>,
) -> (Option<f32>, Option<Uuid>, bool) {
	let Some(candidate_vec) = note_vectors.get(&note_id) else {
		return (None, None, true);
	};
	let mut best_similarity: Option<f32> = None;
	let mut nearest_note_id: Option<Uuid> = None;

	for selected_idx in selected_indices {
		let selected_note_id = candidates[*selected_idx].item.note.note_id;
		let Some(selected_vec) = note_vectors.get(&selected_note_id) else {
			continue;
		};
		let Some(similarity) = cosine_similarity(candidate_vec, selected_vec) else {
			continue;
		};

		if best_similarity.map(|value| similarity > value).unwrap_or(true) {
			best_similarity = Some(similarity);
			nearest_note_id = Some(selected_note_id);
		}
	}

	(best_similarity, nearest_note_id, false)
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

pub fn attach_diversity_decisions_to_trace_candidates(
	candidates: &mut [TraceCandidateRecord],
	decisions: &HashMap<Uuid, DiversityDecision>,
) {
	for candidate in candidates {
		let Some(decision) = decisions.get(&candidate.note_id) else { continue };
		let mut snapshot = candidate.candidate_snapshot.clone();
		let Some(object) = snapshot.as_object_mut() else { continue };

		object.insert("diversity_selected".to_string(), serde_json::json!(decision.selected));
		object.insert(
			"diversity_selected_rank".to_string(),
			serde_json::json!(decision.selected_rank),
		);
		object.insert(
			"diversity_selected_reason".to_string(),
			serde_json::json!(decision.selected_reason),
		);
		object.insert(
			"diversity_skipped_reason".to_string(),
			serde_json::json!(decision.skipped_reason),
		);
		object.insert(
			"diversity_nearest_selected_note_id".to_string(),
			serde_json::json!(decision.nearest_selected_note_id),
		);
		object.insert("diversity_similarity".to_string(), serde_json::json!(decision.similarity));
		object.insert("diversity_mmr_score".to_string(), serde_json::json!(decision.mmr_score));
		object.insert(
			"diversity_missing_embedding".to_string(),
			serde_json::json!(decision.missing_embedding),
		);

		candidate.candidate_snapshot = snapshot;
	}
}

pub fn extract_replay_diversity_decisions(
	candidates: &[TraceReplayCandidate],
) -> HashMap<Uuid, DiversityDecision> {
	let mut out: HashMap<Uuid, DiversityDecision> = HashMap::new();

	for candidate in candidates {
		let has_diversity = candidate.diversity_selected.is_some()
			|| candidate.diversity_selected_rank.is_some()
			|| candidate.diversity_selected_reason.is_some()
			|| candidate.diversity_skipped_reason.is_some()
			|| candidate.diversity_nearest_selected_note_id.is_some()
			|| candidate.diversity_similarity.is_some()
			|| candidate.diversity_mmr_score.is_some()
			|| candidate.diversity_missing_embedding.is_some();

		if !has_diversity {
			continue;
		}

		let selected = candidate.diversity_selected.unwrap_or(false);
		let decision = DiversityDecision {
			selected,
			selected_rank: candidate.diversity_selected_rank,
			selected_reason: candidate
				.diversity_selected_reason
				.clone()
				.unwrap_or_else(|| "replay_selected".to_string()),
			skipped_reason: candidate.diversity_skipped_reason.clone(),
			nearest_selected_note_id: candidate.diversity_nearest_selected_note_id,
			similarity: candidate.diversity_similarity,
			mmr_score: candidate.diversity_mmr_score,
			missing_embedding: candidate.diversity_missing_embedding.unwrap_or(false),
		};
		let replace = match out.get(&candidate.note_id) {
			None => true,
			Some(existing) =>
				if decision.selected != existing.selected {
					decision.selected
				} else {
					let lhs = decision.selected_rank.unwrap_or(u32::MAX);
					let rhs = existing.selected_rank.unwrap_or(u32::MAX);

					lhs < rhs
				},
		};

		if replace {
			out.insert(candidate.note_id, decision);
		}
	}

	out
}

pub fn build_rerank_ranks(items: &[ChunkSnippet], scores: &[f32]) -> Vec<u32> {
	let n = items.len();

	if n == 0 {
		return Vec::new();
	}

	let mut idxs: Vec<usize> = (0..n).collect();

	idxs.sort_by(|&a, &b| {
		let score_a = scores.get(a).copied().unwrap_or(f32::NAN);
		let score_b = scores.get(b).copied().unwrap_or(f32::NAN);
		let ord = retrieval::cmp_f32_desc(score_a, score_b);

		if ord != Ordering::Equal {
			return ord;
		}
		if items[a].note.note_id == items[b].note.note_id {
			let ord = items[a].chunk.chunk_index.cmp(&items[b].chunk.chunk_index);

			if ord != Ordering::Equal {
				return ord;
			}
		}

		let ord = items[a].retrieval_rank.cmp(&items[b].retrieval_rank);

		if ord != Ordering::Equal {
			return ord;
		}

		items[a].chunk.chunk_id.cmp(&items[b].chunk.chunk_id)
	});

	let mut ranks = vec![0_u32; n];

	for (pos, idx) in idxs.into_iter().enumerate() {
		ranks[idx] = pos as u32 + 1;
	}

	ranks
}

pub fn build_rerank_ranks_for_replay(candidates: &[TraceReplayCandidate]) -> Vec<u32> {
	let n = candidates.len();

	if n == 0 {
		return Vec::new();
	}

	let mut idxs: Vec<usize> = (0..n).collect();

	idxs.sort_by(|&a, &b| {
		let score_a = candidates.get(a).map(|candidate| candidate.rerank_score).unwrap_or(f32::NAN);
		let score_b = candidates.get(b).map(|candidate| candidate.rerank_score).unwrap_or(f32::NAN);
		let ord = retrieval::cmp_f32_desc(score_a, score_b);

		if ord != Ordering::Equal {
			return ord;
		}

		let ra = candidates.get(a).map(|candidate| candidate.retrieval_rank).unwrap_or(0);
		let rb = candidates.get(b).map(|candidate| candidate.retrieval_rank).unwrap_or(0);
		let ord = ra.cmp(&rb);

		if ord != Ordering::Equal {
			return ord;
		}

		let na = candidates.get(a).map(|candidate| candidate.note_id).unwrap_or(Uuid::nil());
		let nb = candidates.get(b).map(|candidate| candidate.note_id).unwrap_or(Uuid::nil());
		let ord = na.cmp(&nb);

		if ord != Ordering::Equal {
			return ord;
		}

		let ca = candidates.get(a).map(|candidate| candidate.chunk_id).unwrap_or(Uuid::nil());
		let cb = candidates.get(b).map(|candidate| candidate.chunk_id).unwrap_or(Uuid::nil());

		ca.cmp(&cb)
	});

	let mut ranks = vec![0_u32; n];

	for (pos, idx) in idxs.into_iter().enumerate() {
		ranks[idx] = pos as u32 + 1;
	}

	ranks
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
