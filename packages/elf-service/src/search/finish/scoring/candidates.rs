use crate::search::{
	ChunkCandidate, ElfService, HashMap, NoteMeta, SearchFilter, SearchFilterImpact, Uuid, ranking,
};

impl ElfService {
	pub(in crate::search) fn apply_filter_to_candidates(
		&self,
		candidates: Vec<ChunkCandidate>,
		note_meta: &HashMap<Uuid, NoteMeta>,
		filter: Option<&SearchFilter>,
		requested_candidate_k: u32,
		effective_candidate_k: u32,
	) -> (Vec<ChunkCandidate>, Option<SearchFilterImpact>) {
		let filtered_candidates: Vec<ChunkCandidate> = candidates
			.into_iter()
			.filter(|candidate| ranking::candidate_matches_note(note_meta, candidate))
			.collect();

		match filter {
			Some(filter) => {
				let (candidates, filter_impact) = filter.eval(
					filtered_candidates,
					note_meta,
					requested_candidate_k,
					effective_candidate_k,
				);

				(candidates, Some(filter_impact))
			},
			None => (filtered_candidates, None),
		}
	}
}
