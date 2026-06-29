use std::cmp::Ordering;

use uuid::Uuid;

use crate::search::{ChunkSnippet, TraceReplayCandidate, ranking::retrieval};

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
