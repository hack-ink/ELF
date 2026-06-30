use std::collections::HashMap;

use uuid::Uuid;

use crate::search::{DiversityDecision, ScoredChunk};

pub(super) fn select_diverse_results_disabled(
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
