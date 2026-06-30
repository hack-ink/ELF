use std::path::PathBuf;

use crate::search::{
	self, BlendRankingOverride, OffsetDateTime, RankingRequestOverride,
	RetrievalSourcesRankingOverride, TraceReplayCandidate, TraceReplayContext, Uuid,
};
use elf_config::Config;

fn parse_example_config() -> Config {
	let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
	let path = root_dir.join("elf.example.toml");

	elf_config::load(&path).expect("elf.example.toml must remain parseable and valid.")
}

#[test]
fn ranking_policy_id_is_stable_and_has_expected_format() {
	let cfg = parse_example_config();
	let id_a = search::ranking_policy_id(&cfg, None).expect("Expected policy id.");
	let id_b = search::ranking_policy_id(&cfg, None).expect("Expected policy id.");

	assert_eq!(id_a, id_b);
	assert!(id_a.starts_with("ranking_v2:"), "Unexpected policy id: {id_a}");
	assert_eq!(id_a.len(), "ranking_v2:".len() + 12, "Unexpected policy id: {id_a}");
}

#[test]
fn ranking_policy_id_changes_with_override() {
	let cfg = parse_example_config();
	let base = search::ranking_policy_id(&cfg, None).expect("Expected base policy id.");
	let override_ = RankingRequestOverride {
		blend: Some(BlendRankingOverride {
			enabled: Some(false),
			rerank_normalization: None,
			retrieval_normalization: None,
			segments: None,
		}),
		diversity: None,
		retrieval_sources: None,
	};
	let overridden =
		search::ranking_policy_id(&cfg, Some(&override_)).expect("Expected overridden policy id.");

	assert_ne!(base, overridden);
}

#[test]
fn ranking_policy_id_changes_with_retrieval_source_override() {
	let cfg = parse_example_config();
	let base = search::ranking_policy_id(&cfg, None).expect("Expected base policy id.");
	let override_ = RankingRequestOverride {
		blend: None,
		diversity: None,
		retrieval_sources: Some(RetrievalSourcesRankingOverride {
			fusion_weight: Some(0.75),
			structured_field_weight: Some(1.25),
			recursive_weight: Some(0.0),
			fusion_priority: Some(2),
			structured_field_priority: Some(1),
			recursive_priority: Some(0),
		}),
	};
	let overridden =
		search::ranking_policy_id(&cfg, Some(&override_)).expect("Expected overridden policy id.");

	assert_ne!(base, overridden);
}

#[test]
fn replay_ranking_policy_id_matches_ranking_policy_id() {
	let cfg = parse_example_config();
	let expected = search::ranking_policy_id(&cfg, None).expect("Expected policy id.");
	let now = OffsetDateTime::from_unix_timestamp(0).expect("Valid timestamp.");
	let trace = TraceReplayContext {
		trace_id: Uuid::new_v4(),
		query: "deployment steps".to_string(),
		candidate_count: 3,
		top_k: 2,
		created_at: now,
	};
	let candidates = vec![
		TraceReplayCandidate {
			note_id: Uuid::new_v4(),
			chunk_id: Uuid::new_v4(),
			chunk_index: 0,
			snippet: "deployment steps".to_string(),
			retrieval_rank: 1,
			retrieval_score: None,
			rerank_score: 0.1,
			note_scope: "project_shared".to_string(),
			note_importance: 0.1,
			note_updated_at: now,
			note_hit_count: 0,
			note_last_hit_at: None,
			diversity_selected: None,
			diversity_selected_rank: None,
			diversity_selected_reason: None,
			diversity_skipped_reason: None,
			diversity_nearest_selected_note_id: None,
			diversity_similarity: None,
			diversity_mmr_score: None,
			diversity_missing_embedding: None,
		},
		TraceReplayCandidate {
			note_id: Uuid::new_v4(),
			chunk_id: Uuid::new_v4(),
			chunk_index: 0,
			snippet: "deployment steps".to_string(),
			retrieval_rank: 2,
			retrieval_score: None,
			rerank_score: 0.9,
			note_scope: "project_shared".to_string(),
			note_importance: 0.1,
			note_updated_at: now,
			note_hit_count: 0,
			note_last_hit_at: None,
			diversity_selected: None,
			diversity_selected_rank: None,
			diversity_selected_reason: None,
			diversity_skipped_reason: None,
			diversity_nearest_selected_note_id: None,
			diversity_similarity: None,
			diversity_mmr_score: None,
			diversity_missing_embedding: None,
		},
		TraceReplayCandidate {
			note_id: Uuid::new_v4(),
			chunk_id: Uuid::new_v4(),
			chunk_index: 0,
			snippet: "deployment steps".to_string(),
			retrieval_rank: 3,
			retrieval_score: None,
			rerank_score: 0.2,
			note_scope: "org_shared".to_string(),
			note_importance: 0.1,
			note_updated_at: now,
			note_hit_count: 0,
			note_last_hit_at: None,
			diversity_selected: None,
			diversity_selected_rank: None,
			diversity_selected_reason: None,
			diversity_skipped_reason: None,
			diversity_nearest_selected_note_id: None,
			diversity_similarity: None,
			diversity_mmr_score: None,
			diversity_missing_embedding: None,
		},
	];
	let out = search::replay_ranking_from_candidates(&cfg, &trace, None, &candidates, 2)
		.expect("Expected replay output.");

	for item in out {
		assert_eq!(item.explain.ranking.policy_id, expected);
	}
}
