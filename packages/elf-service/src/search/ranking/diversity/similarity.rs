use std::collections::HashMap;

use uuid::Uuid;

use crate::search::ScoredChunk;

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
