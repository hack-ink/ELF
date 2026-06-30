use crate::search::{
	self, DiversityDecision, ElfService, HashMap, ResolvedDiversityPolicy, Result, ScoredChunk,
	Uuid, ranking,
};

impl ElfService {
	pub(in crate::search) async fn apply_diversity_policy(
		&self,
		results: Vec<ScoredChunk>,
		top_k: u32,
		diversity_policy: &ResolvedDiversityPolicy,
	) -> Result<(Vec<ScoredChunk>, HashMap<Uuid, DiversityDecision>)> {
		let note_vectors = if diversity_policy.enabled {
			search::fetch_note_vectors_for_diversity(&self.db.pool, results.as_slice()).await?
		} else {
			HashMap::new()
		};
		let (selected_results, diversity_decisions) =
			ranking::select_diverse_results(results, top_k, diversity_policy, &note_vectors);

		Ok((selected_results, diversity_decisions))
	}
}
