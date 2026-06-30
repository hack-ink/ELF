mod disabled;
mod enabled;
mod pick;

use std::collections::HashMap;

use uuid::Uuid;

use crate::search::{DiversityDecision, ScoredChunk, ranking::policy::ResolvedDiversityPolicy};

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
		return disabled::select_diverse_results_disabled(candidates, top_k, note_vectors);
	}

	enabled::select_diverse_results_enabled(candidates, top_k, policy, note_vectors)
}
