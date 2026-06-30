mod merge;
mod payload;

pub use self::merge::merge_retrieval_candidates;

use std::{
	cmp::Ordering,
	collections::{HashMap, HashSet},
};

use qdrant_client::qdrant::ScoredPoint;
use uuid::Uuid;

use crate::search::{ChunkCandidate, ChunkRow, NoteMeta};

pub fn collect_chunk_candidates(
	points: &[ScoredPoint],
	max_candidates: u32,
	candidate_k: u32,
) -> Vec<ChunkCandidate> {
	let limit = if max_candidates == 0 || max_candidates >= candidate_k {
		points.len()
	} else {
		max_candidates as usize
	};
	let mut out = Vec::new();
	let mut seen = HashSet::new();

	for (idx, point) in points.iter().take(limit).enumerate() {
		let chunk_id = point
			.id
			.as_ref()
			.and_then(payload::point_id_to_uuid)
			.or_else(|| payload::payload_uuid(&point.payload, "chunk_id"));
		let Some(chunk_id) = chunk_id else {
			tracing::warn!("Chunk candidate missing chunk_id.");

			continue;
		};

		if !seen.insert(chunk_id) {
			continue;
		}

		let Some(note_id) = payload::payload_uuid(&point.payload, "note_id") else {
			tracing::warn!(chunk_id = %chunk_id, "Chunk candidate missing note_id.");

			continue;
		};
		let Some(chunk_index) = payload::payload_i32(&point.payload, "chunk_index") else {
			tracing::warn!(chunk_id = %chunk_id, "Chunk candidate missing chunk_index.");

			continue;
		};
		let updated_at = payload::payload_rfc3339(&point.payload, "updated_at");
		let embedding_version = payload::payload_string(&point.payload, "embedding_version");
		let scope = payload::payload_string(&point.payload, "scope");

		out.push(ChunkCandidate {
			chunk_id,
			note_id,
			chunk_index,
			retrieval_rank: idx as u32 + 1,
			retrieval_score: Some(point.score),
			updated_at,
			embedding_version,
			scope,
		});
	}

	out
}

pub fn candidate_matches_note(
	note_meta: &HashMap<Uuid, NoteMeta>,
	candidate: &ChunkCandidate,
) -> bool {
	let Some(note) = note_meta.get(&candidate.note_id) else { return false };

	if let Some(version) = candidate.embedding_version.as_deref()
		&& version != note.embedding_version.as_str()
	{
		return false;
	}
	if let Some(ts) = candidate.updated_at
		&& ts != note.updated_at
	{
		return false;
	}

	true
}

pub fn collect_neighbor_pairs(candidates: &[ChunkCandidate]) -> Vec<(Uuid, i32)> {
	let mut seen = HashSet::new();
	let mut out = Vec::new();

	for candidate in candidates {
		let mut indices = Vec::with_capacity(3);

		indices.push(candidate.chunk_index);

		if let Some(prev) = candidate.chunk_index.checked_sub(1) {
			indices.push(prev);
		}
		if let Some(next) = candidate.chunk_index.checked_add(1) {
			indices.push(next);
		}

		for idx in indices {
			let key = (candidate.note_id, idx);

			if seen.insert(key) {
				out.push(key);
			}
		}
	}

	out
}

pub fn stitch_snippet(
	note_id: Uuid,
	chunk_index: i32,
	chunks: &HashMap<(Uuid, i32), ChunkRow>,
) -> String {
	let indices = [chunk_index.checked_sub(1), Some(chunk_index), chunk_index.checked_add(1)];
	let mut out = String::new();

	for index in indices.into_iter().flatten() {
		if let Some(chunk) = chunks.get(&(note_id, index)) {
			out.push_str(chunk.text.as_str());
		}
	}

	out.trim().to_string()
}

pub fn rank_normalize(rank: u32, total: u32) -> f32 {
	if total <= 1 {
		return 1.0;
	}
	if rank == 0 {
		return 0.0;
	}

	let denom = (total - 1) as f32;
	let pos = (rank.saturating_sub(1)) as f32;

	(1.0 - pos / denom).clamp(0.0, 1.0)
}

pub fn cmp_f32_desc(a: f32, b: f32) -> Ordering {
	match (a.is_nan(), b.is_nan()) {
		(true, true) => Ordering::Equal,
		(true, false) => Ordering::Greater,
		(false, true) => Ordering::Less,
		(false, false) => b.partial_cmp(&a).unwrap_or(Ordering::Equal),
	}
}
