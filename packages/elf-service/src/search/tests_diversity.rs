use crate::search::{
	ChunkMeta, ChunkSnippet, HashMap, NoteMeta, OffsetDateTime, ScoredChunk, TraceReplayCandidate,
	Uuid,
	ranking::{self, ResolvedDiversityPolicy},
};

fn test_scored_chunk(note_id: Uuid, retrieval_rank: u32, now: OffsetDateTime) -> ScoredChunk {
	let note = NoteMeta {
		note_id,
		note_type: "fact".to_string(),
		key: None,
		scope: "project_shared".to_string(),
		agent_id: "agent-a".to_string(),
		importance: 0.1,
		confidence: 0.9,
		updated_at: now,
		expires_at: None,
		source_ref: serde_json::json!({}),
		embedding_version: "v1".to_string(),
		hit_count: 0,
		last_hit_at: None,
	};
	let chunk = ChunkMeta {
		chunk_id: Uuid::new_v4(),
		chunk_index: i32::try_from(retrieval_rank.saturating_sub(1)).unwrap_or(0),
		start_offset: 0,
		end_offset: 16,
	};
	let item = ChunkSnippet {
		note,
		chunk,
		snippet: format!("snippet-{retrieval_rank}"),
		retrieval_rank,
		retrieval_score: None,
	};

	ScoredChunk {
		item,
		final_score: 0.0,
		rerank_score: 0.0,
		rerank_rank: retrieval_rank,
		rerank_norm: 0.0,
		retrieval_norm: 0.0,
		blend_retrieval_weight: 0.5,
		retrieval_term: 0.0,
		rerank_term: 0.0,
		tie_breaker_score: 0.0,
		scope_context_boost: 0.0,
		age_days: 0.0,
		importance: 0.1,
		deterministic_lexical_overlap_ratio: 0.0,
		deterministic_lexical_bonus: 0.0,
		deterministic_hit_count: 0,
		deterministic_last_hit_age_days: None,
		deterministic_hit_boost: 0.0,
		deterministic_decay_penalty: 0.0,
	}
}

#[test]
fn diversity_selection_skips_high_similarity_when_alternative_exists() {
	let now = OffsetDateTime::from_unix_timestamp(0).expect("Valid timestamp.");
	let note_a = Uuid::new_v4();
	let note_b = Uuid::new_v4();
	let note_c = Uuid::new_v4();
	let candidates = vec![
		test_scored_chunk(note_a, 1, now),
		test_scored_chunk(note_b, 2, now),
		test_scored_chunk(note_c, 3, now),
	];
	let mut vectors = HashMap::new();

	vectors.insert(note_a, vec![1.0, 0.0]);
	vectors.insert(note_b, vec![0.99, 0.01]);
	vectors.insert(note_c, vec![0.0, 1.0]);

	let policy = ResolvedDiversityPolicy {
		enabled: true,
		sim_threshold: 0.9,
		mmr_lambda: 0.7,
		max_skips: 64,
	};
	let (selected, decisions) = ranking::select_diverse_results(candidates, 2, &policy, &vectors);
	let selected_ids: Vec<Uuid> = selected.iter().map(|item| item.item.note.note_id).collect();

	assert_eq!(selected_ids, vec![note_a, note_c]);
	assert_eq!(
		decisions.get(&note_b).and_then(|decision| decision.skipped_reason.as_deref()),
		Some("similarity_threshold")
	);
}

#[test]
fn diversity_selection_backfills_when_max_skips_is_reached() {
	let now = OffsetDateTime::from_unix_timestamp(0).expect("Valid timestamp.");
	let note_a = Uuid::new_v4();
	let note_b = Uuid::new_v4();
	let candidates = vec![test_scored_chunk(note_a, 1, now), test_scored_chunk(note_b, 2, now)];
	let mut vectors = HashMap::new();

	vectors.insert(note_a, vec![1.0, 0.0]);
	vectors.insert(note_b, vec![0.99, 0.01]);

	let policy = ResolvedDiversityPolicy {
		enabled: true,
		sim_threshold: 0.9,
		mmr_lambda: 0.7,
		max_skips: 0,
	};
	let (selected, decisions) = ranking::select_diverse_results(candidates, 2, &policy, &vectors);
	let selected_ids: Vec<Uuid> = selected.iter().map(|item| item.item.note.note_id).collect();
	let selected_reason = decisions.get(&note_b).map(|decision| decision.selected_reason.as_str());

	assert_eq!(selected_ids, vec![note_a, note_b]);
	assert_eq!(selected_reason, Some("max_skips_backfill"));
}

#[test]
fn replay_diversity_decisions_prefer_selected_entry_for_same_note() {
	let now = OffsetDateTime::from_unix_timestamp(0).expect("Valid timestamp.");
	let note_id = Uuid::new_v4();
	let first = TraceReplayCandidate {
		note_id,
		chunk_id: Uuid::new_v4(),
		chunk_index: 0,
		snippet: "first".to_string(),
		retrieval_rank: 2,
		retrieval_score: None,
		rerank_score: 0.2,
		note_scope: "project_shared".to_string(),
		note_importance: 0.1,
		note_updated_at: now,
		note_hit_count: 0,
		note_last_hit_at: None,
		diversity_selected: Some(false),
		diversity_selected_rank: None,
		diversity_selected_reason: Some("not_selected".to_string()),
		diversity_skipped_reason: Some("lower_mmr".to_string()),
		diversity_nearest_selected_note_id: None,
		diversity_similarity: Some(0.95),
		diversity_mmr_score: Some(0.12),
		diversity_missing_embedding: Some(false),
	};
	let second = TraceReplayCandidate {
		note_id,
		chunk_id: Uuid::new_v4(),
		chunk_index: 1,
		snippet: "second".to_string(),
		retrieval_rank: 1,
		retrieval_score: None,
		rerank_score: 0.3,
		note_scope: "project_shared".to_string(),
		note_importance: 0.1,
		note_updated_at: now,
		note_hit_count: 0,
		note_last_hit_at: None,
		diversity_selected: Some(true),
		diversity_selected_rank: Some(2),
		diversity_selected_reason: Some("mmr".to_string()),
		diversity_skipped_reason: None,
		diversity_nearest_selected_note_id: None,
		diversity_similarity: Some(0.35),
		diversity_mmr_score: Some(0.44),
		diversity_missing_embedding: Some(false),
	};
	let decisions = ranking::extract_replay_diversity_decisions(&[first, second]);
	let decision = decisions.get(&note_id).expect("Expected merged decision.");

	assert!(decision.selected);
	assert_eq!(decision.selected_rank, Some(2));
	assert_eq!(decision.selected_reason, "mmr");
}
