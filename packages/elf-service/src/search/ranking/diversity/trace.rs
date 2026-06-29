use std::collections::HashMap;

use uuid::Uuid;

use crate::search::{
	DiversityDecision, SearchDiversityExplain, TraceCandidateRecord, TraceReplayCandidate,
};

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
