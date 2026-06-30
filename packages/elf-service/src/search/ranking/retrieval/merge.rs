use std::{
	cmp::Ordering,
	collections::{HashMap, HashSet},
};

use uuid::Uuid;

use crate::search::{
	ChunkCandidate, RetrievalSourceCandidates, RetrievalSourceKind,
	ranking::policy::ResolvedRetrievalSourcesPolicy,
};

#[derive(Debug)]
struct MergedRetrievalCandidate {
	candidate: ChunkCandidate,
	source_ranks: HashMap<RetrievalSourceKind, u32>,
	combined_score: f32,
}

pub fn merge_retrieval_candidates(
	sources: Vec<RetrievalSourceCandidates>,
	policy: &ResolvedRetrievalSourcesPolicy,
	candidate_k: u32,
) -> Vec<ChunkCandidate> {
	if candidate_k == 0 {
		return Vec::new();
	}

	let mut by_chunk: HashMap<Uuid, MergedRetrievalCandidate> = HashMap::new();
	let mut source_totals: HashMap<RetrievalSourceKind, u32> = HashMap::new();

	for source in sources {
		let mut seen_for_source = HashSet::new();

		for candidate in &source.candidates {
			if seen_for_source.insert(candidate.chunk_id) {
				*source_totals.entry(source.source).or_insert(0) += 1;
			}
		}
		for candidate in source.candidates {
			let chunk_id = candidate.chunk_id;
			let rank = candidate.retrieval_rank;

			match by_chunk.get_mut(&chunk_id) {
				Some(existing) => {
					let entry = existing.source_ranks.entry(source.source).or_insert(rank);

					*entry = (*entry).min(rank);
				},
				None => {
					let mut source_ranks = HashMap::new();

					source_ranks.insert(source.source, rank);
					by_chunk.insert(
						chunk_id,
						MergedRetrievalCandidate { candidate, source_ranks, combined_score: 0.0 },
					);
				},
			}
		}
	}

	if by_chunk.is_empty() {
		return Vec::new();
	}

	for total in source_totals.values_mut() {
		*total = (*total).max(1);
	}

	let mut source_order: Vec<RetrievalSourceKind> = source_totals.keys().copied().collect();

	source_order.sort_by(|left, right| {
		retrieval_source_priority(policy, *left)
			.cmp(&retrieval_source_priority(policy, *right))
			.then_with(|| {
				retrieval_source_kind_order(*left).cmp(&retrieval_source_kind_order(*right))
			})
	});

	let mut merged: Vec<MergedRetrievalCandidate> = by_chunk.into_values().collect();

	for candidate in &mut merged {
		let mut combined_score = 0.0_f32;

		for (source, rank) in &candidate.source_ranks {
			let total = source_totals.get(source).copied().unwrap_or(1);

			combined_score +=
				retrieval_source_weight(policy, *source) * super::rank_normalize(*rank, total);
		}

		candidate.combined_score = combined_score;
	}

	merged.sort_by(|left, right| {
		super::cmp_f32_desc(left.combined_score, right.combined_score)
			.then_with(|| right.source_ranks.len().cmp(&left.source_ranks.len()))
			.then_with(|| {
				for source in &source_order {
					let lhs = left.source_ranks.get(source).copied();
					let rhs = right.source_ranks.get(source).copied();
					let ord = rank_asc(lhs, rhs);

					if ord != Ordering::Equal {
						return ord;
					}
				}

				Ordering::Equal
			})
			.then_with(|| left.candidate.chunk_id.cmp(&right.candidate.chunk_id))
	});

	let mut out = Vec::new();

	for (idx, mut candidate) in merged.into_iter().take(candidate_k as usize).enumerate() {
		candidate.candidate.retrieval_rank = idx as u32 + 1;
		candidate.candidate.retrieval_score = Some(candidate.combined_score);

		out.push(candidate.candidate);
	}

	out
}

fn retrieval_source_weight(
	policy: &ResolvedRetrievalSourcesPolicy,
	source: RetrievalSourceKind,
) -> f32 {
	match source {
		RetrievalSourceKind::Fusion => policy.fusion_weight,
		RetrievalSourceKind::StructuredField => policy.structured_field_weight,
		RetrievalSourceKind::Recursive => policy.recursive_weight,
	}
}

fn retrieval_source_priority(
	policy: &ResolvedRetrievalSourcesPolicy,
	source: RetrievalSourceKind,
) -> u32 {
	match source {
		RetrievalSourceKind::StructuredField => policy.structured_field_priority,
		RetrievalSourceKind::Fusion => policy.fusion_priority,
		RetrievalSourceKind::Recursive => policy.recursive_priority,
	}
}

fn retrieval_source_kind_order(source: RetrievalSourceKind) -> u8 {
	match source {
		RetrievalSourceKind::StructuredField => 0,
		RetrievalSourceKind::Fusion => 1,
		RetrievalSourceKind::Recursive => 2,
	}
}

fn rank_asc(left: Option<u32>, right: Option<u32>) -> Ordering {
	let lhs = left.unwrap_or(u32::MAX);
	let rhs = right.unwrap_or(u32::MAX);

	lhs.cmp(&rhs)
}
