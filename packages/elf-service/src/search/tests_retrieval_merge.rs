use crate::search::{
	ChunkCandidate, RetrievalSourceCandidates, RetrievalSourceKind, Uuid, ranking,
};

fn test_chunk_candidate(note_id: Uuid, retrieval_rank: u32) -> ChunkCandidate {
	ChunkCandidate {
		chunk_id: Uuid::new_v4(),
		note_id,
		chunk_index: 0,
		retrieval_rank,
		retrieval_score: None,
		scope: None,
		updated_at: None,
		embedding_version: Some("v1".to_string()),
	}
}

fn default_retrieval_sources_policy() -> ranking::ResolvedRetrievalSourcesPolicy {
	ranking::ResolvedRetrievalSourcesPolicy {
		fusion_weight: 1.0,
		structured_field_weight: 1.0,
		recursive_weight: 0.0,
		fusion_priority: 1,
		structured_field_priority: 0,
		recursive_priority: 0,
	}
}

#[test]
fn merge_retrieval_candidates_keeps_structured_hits_under_full_fusion_capacity() {
	let mut fusion = Vec::new();

	for rank in 1..=10 {
		fusion.push(test_chunk_candidate(Uuid::new_v4(), rank));
	}

	let structured = vec![test_chunk_candidate(Uuid::new_v4(), 1)];
	let structured_chunk_id = structured[0].chunk_id;
	let merged = ranking::merge_retrieval_candidates(
		vec![
			RetrievalSourceCandidates { source: RetrievalSourceKind::Fusion, candidates: fusion },
			RetrievalSourceCandidates {
				source: RetrievalSourceKind::StructuredField,
				candidates: structured,
			},
		],
		&default_retrieval_sources_policy(),
		10,
	);
	let merged_chunk_ids: Vec<Uuid> = merged.iter().map(|candidate| candidate.chunk_id).collect();

	assert!(
		merged_chunk_ids.contains(&structured_chunk_id),
		"Structured candidate was dropped by retrieval fusion."
	);
}

#[test]
fn merge_retrieval_candidates_prefers_dual_source_signal_on_tie() {
	let shared_note_id = Uuid::new_v4();
	let shared_chunk_id = Uuid::new_v4();
	let fusion_only_note_id = Uuid::new_v4();
	let fusion_only_chunk_id = Uuid::new_v4();
	let fusion = vec![
		ChunkCandidate {
			chunk_id: shared_chunk_id,
			note_id: shared_note_id,
			chunk_index: 0,
			retrieval_rank: 9,
			retrieval_score: None,
			scope: None,
			updated_at: None,
			embedding_version: Some("v1".to_string()),
		},
		ChunkCandidate {
			chunk_id: fusion_only_chunk_id,
			note_id: fusion_only_note_id,
			chunk_index: 0,
			retrieval_rank: 1,
			retrieval_score: None,
			scope: None,
			updated_at: None,
			embedding_version: Some("v1".to_string()),
		},
	];
	let structured = vec![ChunkCandidate {
		chunk_id: shared_chunk_id,
		note_id: shared_note_id,
		chunk_index: 0,
		retrieval_rank: 1,
		retrieval_score: None,
		scope: None,
		updated_at: None,
		embedding_version: Some("v1".to_string()),
	}];
	let merged = ranking::merge_retrieval_candidates(
		vec![
			RetrievalSourceCandidates { source: RetrievalSourceKind::Fusion, candidates: fusion },
			RetrievalSourceCandidates {
				source: RetrievalSourceKind::StructuredField,
				candidates: structured,
			},
		],
		&default_retrieval_sources_policy(),
		1,
	);
	let first = merged.first().expect("Expected merged candidate.");

	assert_eq!(first.chunk_id, shared_chunk_id);
}

#[test]
fn merge_retrieval_candidates_uses_configured_source_priority_on_tie() {
	let fusion_chunk_id = Uuid::from_u128(1);
	let recursive_chunk_id = Uuid::from_u128(2);
	let mut fusion_candidate = test_chunk_candidate(Uuid::new_v4(), 1);
	let mut recursive_candidate = test_chunk_candidate(Uuid::new_v4(), 1);

	fusion_candidate.chunk_id = fusion_chunk_id;
	recursive_candidate.chunk_id = recursive_chunk_id;

	let policy = ranking::ResolvedRetrievalSourcesPolicy {
		fusion_weight: 1.0,
		structured_field_weight: 0.0,
		recursive_weight: 1.0,
		fusion_priority: 10,
		structured_field_priority: 20,
		recursive_priority: 0,
	};
	let merged = ranking::merge_retrieval_candidates(
		vec![
			RetrievalSourceCandidates {
				source: RetrievalSourceKind::Fusion,
				candidates: vec![fusion_candidate],
			},
			RetrievalSourceCandidates {
				source: RetrievalSourceKind::Recursive,
				candidates: vec![recursive_candidate],
			},
		],
		&policy,
		2,
	);

	assert_eq!(merged[0].chunk_id, recursive_chunk_id);
	assert_eq!(merged[1].chunk_id, fusion_chunk_id);
}

#[test]
fn retrieval_weight_for_rank_uses_first_matching_segment_or_last() {
	let segments = vec![
		ranking::BlendSegment { max_retrieval_rank: 3, retrieval_weight: 0.7 },
		ranking::BlendSegment { max_retrieval_rank: 10, retrieval_weight: 0.2 },
	];

	assert!((ranking::retrieval_weight_for_rank(1, &segments) - 0.7).abs() < 1e-6);
	assert!((ranking::retrieval_weight_for_rank(3, &segments) - 0.7).abs() < 1e-6);
	assert!((ranking::retrieval_weight_for_rank(4, &segments) - 0.2).abs() < 1e-6);
	assert!((ranking::retrieval_weight_for_rank(999, &segments) - 0.2).abs() < 1e-6);
}

#[test]
fn blend_math_is_linear_and_additive() {
	let segments = vec![
		ranking::BlendSegment { max_retrieval_rank: 2, retrieval_weight: 0.7 },
		ranking::BlendSegment { max_retrieval_rank: 10, retrieval_weight: 0.2 },
	];
	let retrieval_rank = 3;
	let rerank_rank = 2;
	let retrieval_norm = ranking::rank_normalize(retrieval_rank, 10);
	let rerank_norm = ranking::rank_normalize(rerank_rank, 4);
	let blend_retrieval_weight = ranking::retrieval_weight_for_rank(retrieval_rank, &segments);

	assert!((blend_retrieval_weight - 0.2).abs() < 1e-6);
	assert!((retrieval_norm - (7.0 / 9.0)).abs() < 1e-6);
	assert!((rerank_norm - (2.0 / 3.0)).abs() < 1e-6);

	let retrieval_term = blend_retrieval_weight * retrieval_norm;
	let rerank_term = (1.0 - blend_retrieval_weight) * rerank_norm;
	let tie_breaker_score = 0.1;
	let scope_context_boost = 0.0;
	let final_score = retrieval_term + rerank_term + tie_breaker_score + scope_context_boost;
	let expected = (0.2 * (7.0 / 9.0)) + (0.8 * (2.0 / 3.0)) + 0.1;

	assert!((final_score - expected).abs() < 1e-6, "Unexpected final_score: {final_score}");
}
