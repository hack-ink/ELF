use std::{
	cmp::Ordering,
	collections::{HashMap, HashSet},
};

use qdrant_client::qdrant::{PointId, ScoredPoint, Value, point_id::PointIdOptions, value::Kind};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use super::policy::ResolvedRetrievalSourcesPolicy;
use crate::search::{
	ChunkCandidate, ChunkRow, NoteMeta, RetrievalSourceCandidates, RetrievalSourceKind,
};

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
			.and_then(point_id_to_uuid)
			.or_else(|| payload_uuid(&point.payload, "chunk_id"));
		let Some(chunk_id) = chunk_id else {
			tracing::warn!("Chunk candidate missing chunk_id.");

			continue;
		};

		if !seen.insert(chunk_id) {
			continue;
		}

		let Some(note_id) = payload_uuid(&point.payload, "note_id") else {
			tracing::warn!(chunk_id = %chunk_id, "Chunk candidate missing note_id.");

			continue;
		};
		let Some(chunk_index) = payload_i32(&point.payload, "chunk_index") else {
			tracing::warn!(chunk_id = %chunk_id, "Chunk candidate missing chunk_index.");

			continue;
		};
		let updated_at = payload_rfc3339(&point.payload, "updated_at");
		let embedding_version = payload_string(&point.payload, "embedding_version");

		out.push(ChunkCandidate {
			chunk_id,
			note_id,
			chunk_index,
			retrieval_rank: idx as u32 + 1,
			updated_at,
			embedding_version,
		});
	}

	out
}

pub fn retrieval_source_weight(
	policy: &ResolvedRetrievalSourcesPolicy,
	source: RetrievalSourceKind,
) -> f32 {
	match source {
		RetrievalSourceKind::Fusion => policy.fusion_weight,
		RetrievalSourceKind::StructuredField => policy.structured_field_weight,
	}
}

pub fn retrieval_source_priority(
	policy: &ResolvedRetrievalSourcesPolicy,
	source: RetrievalSourceKind,
) -> u32 {
	match source {
		RetrievalSourceKind::StructuredField => policy.structured_field_priority,
		RetrievalSourceKind::Fusion => policy.fusion_priority,
	}
}

pub fn retrieval_source_kind_order(source: RetrievalSourceKind) -> u8 {
	match source {
		RetrievalSourceKind::StructuredField => 0,
		RetrievalSourceKind::Fusion => 1,
	}
}

pub fn merge_retrieval_candidates(
	sources: Vec<RetrievalSourceCandidates>,
	policy: &ResolvedRetrievalSourcesPolicy,
	candidate_k: u32,
) -> Vec<ChunkCandidate> {
	if candidate_k == 0 {
		return Vec::new();
	}

	#[derive(Debug)]
	struct MergedRetrievalCandidate {
		candidate: ChunkCandidate,
		source_ranks: HashMap<RetrievalSourceKind, u32>,
		combined_score: f32,
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
				retrieval_source_weight(policy, *source) * rank_normalize(*rank, total);
		}
		candidate.combined_score = combined_score;
	}

	merged.sort_by(|left, right| {
		cmp_f32_desc(left.combined_score, right.combined_score)
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

		out.push(candidate.candidate);
	}

	out
}

pub fn rank_asc(left: Option<u32>, right: Option<u32>) -> Ordering {
	let lhs = left.unwrap_or(u32::MAX);
	let rhs = right.unwrap_or(u32::MAX);

	lhs.cmp(&rhs)
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
pub fn point_id_to_uuid(point_id: &PointId) -> Option<Uuid> {
	match &point_id.point_id_options {
		Some(PointIdOptions::Uuid(id)) => Uuid::parse_str(id).ok(),
		_ => None,
	}
}

pub fn payload_uuid(payload: &HashMap<String, Value>, key: &str) -> Option<Uuid> {
	let value = payload.get(key)?;

	match &value.kind {
		Some(Kind::StringValue(text)) => Uuid::parse_str(text).ok(),
		_ => None,
	}
}

pub fn payload_string(payload: &HashMap<String, Value>, key: &str) -> Option<String> {
	let value = payload.get(key)?;

	match &value.kind {
		Some(Kind::StringValue(text)) => Some(text.to_string()),
		_ => None,
	}
}

pub fn payload_rfc3339(payload: &HashMap<String, Value>, key: &str) -> Option<OffsetDateTime> {
	let text = payload_string(payload, key)?;

	OffsetDateTime::parse(text.as_str(), &Rfc3339).ok()
}

pub fn payload_i32(payload: &HashMap<String, Value>, key: &str) -> Option<i32> {
	let value = payload.get(key)?;

	match &value.kind {
		Some(Kind::IntegerValue(value)) => i32::try_from(*value).ok(),
		Some(Kind::DoubleValue(value)) =>
			if value.fract() == 0.0 {
				i32::try_from(*value as i64).ok()
			} else {
				None
			},
		_ => None,
	}
}
