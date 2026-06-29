use std::path::PathBuf;

use serde_json::Value;

use crate::{
	ElfService,
	search::{
		self, BlendRankingOverride, ChunkCandidate, ChunkMeta, ChunkSnippet, HashMap, NoteMeta,
		OffsetDateTime, RankingRequestOverride, RerankCacheCandidate, RerankCacheItem,
		RerankCachePayload, RetrievalSourceCandidates, RetrievalSourceKind,
		RetrievalSourcesRankingOverride, ScoredChunk, TraceReplayCandidate, TraceReplayContext,
		Uuid,
		ranking::{self, ResolvedDiversityPolicy},
	},
};
use elf_config::{Config, SearchDynamic};

#[test]
fn dense_embedding_input_includes_project_context_suffix() {
	let input =
		ranking::build_dense_embedding_input("Find payments code.", Some("This is a billing API."));

	assert!(input.starts_with("Find payments code.\n\nProject context:\n"));
	assert!(input.contains("This is a billing API."));
}

#[test]
fn dense_embedding_input_skips_empty_project_context() {
	let input = ranking::build_dense_embedding_input("Find payments code.", Some("   "));

	assert_eq!(input, "Find payments code.");
}

#[test]
fn scope_description_boost_matches_whole_tokens_only() {
	let tokens = vec!["go".to_string()];
	let boost = ranking::scope_description_boost(&tokens, "MongoDB operational notes.", 0.1);

	assert_eq!(boost, 0.0);
}

#[test]
fn scope_description_boost_scales_by_fraction_of_matched_tokens() {
	let tokens = vec!["security".to_string(), "policy".to_string(), "deployment".to_string()];
	let boost = ranking::scope_description_boost(&tokens, "Security policy notes.", 0.12);

	assert!((boost - 0.08).abs() < 1e-4, "Unexpected boost: {boost}");
}

#[test]
fn normalize_queries_includes_original_and_dedupes() {
	let queries = vec!["alpha".to_string(), "beta".to_string(), "alpha".to_string()];
	let normalized = ranking::normalize_queries(queries, "alpha", true, 4);

	assert_eq!(normalized, vec!["alpha".to_string(), "beta".to_string()]);
}

#[test]
fn normalize_queries_respects_max_queries() {
	let queries =
		vec!["one".to_string(), "two".to_string(), "three".to_string(), "four".to_string()];
	let normalized = ranking::normalize_queries(queries, "zero", true, 3);

	assert_eq!(normalized.len(), 3);
}

#[test]
fn dynamic_trigger_checks_candidates_and_score() {
	let cfg = SearchDynamic { min_candidates: 10, min_top_score: 0.2 };

	assert!(ranking::should_expand_dynamic(5, 0.9, &cfg));
	assert!(ranking::should_expand_dynamic(20, 0.1, &cfg));
	assert!(!ranking::should_expand_dynamic(20, 0.9, &cfg));
}

#[test]
fn rank_normalize_maps_rank_to_unit_interval() {
	assert!((ranking::rank_normalize(1, 1) - 1.0).abs() < 1e-6);
	assert!((ranking::rank_normalize(1, 5) - 1.0).abs() < 1e-6);
	assert!((ranking::rank_normalize(3, 5) - 0.5).abs() < 1e-6);
	assert!((ranking::rank_normalize(5, 5) - 0.0).abs() < 1e-6);
	assert!((ranking::rank_normalize(0, 5) - 0.0).abs() < 1e-6);
}

#[test]
fn build_trace_audit_includes_token_id_when_present() {
	let audit = search::build_trace_audit("agent-a", Some("tok-123"));

	assert_eq!(audit.get("actor_id"), Some(&Value::from("agent-a")));
	assert_eq!(audit.get("token_id"), Some(&Value::from("tok-123")));
}

#[test]
fn build_trace_audit_omits_token_id_when_empty() {
	let audit = search::build_trace_audit("agent-a", Some("   "));

	assert_eq!(audit.get("actor_id"), Some(&Value::from("agent-a")));
	assert!(audit.get("token_id").is_none());
}

#[test]
fn relation_context_rows_without_evidence_are_suppressed() {
	let now = OffsetDateTime::from_unix_timestamp(100).expect("valid timestamp");
	let note_id = Uuid::from_u128(1);
	let contexts =
		ElfService::group_relation_context_rows(vec![search::SearchRelationContextRow {
			note_id,
			fact_id: Uuid::from_u128(2),
			scope: "project_shared".to_string(),
			subject_canonical: Some("Alice".to_string()),
			subject_kind: Some("person".to_string()),
			predicate: "prefers".to_string(),
			object_entity_id: None,
			object_canonical: None,
			object_kind: None,
			object_value: Some("source-bound recall".to_string()),
			valid_from: now,
			valid_to: None,
			is_current: true,
			evidence_note_ids: Vec::new(),
		}]);

	assert!(!contexts.contains_key(&note_id));
}

#[test]
fn relation_context_sql_enforces_shared_grant_keys() {
	assert!(
		search::RELATION_CONTEXT_SQL
			.contains("concat(gf.scope, ':', gf.agent_id) = ANY($10::text[])")
	);
	assert!(
		search::RELATION_CONTEXT_SQL.contains(
			"concat(evidence_note.scope, ':', evidence_note.agent_id) = ANY($10::text[])"
		)
	);
}

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

#[test]
fn expansion_cache_key_changes_with_max_queries() {
	let key_a = ranking::build_expansion_cache_key("alpha", 4, true, "llm", "model", 0.1_f32)
		.expect("Expected cache key.");
	let key_b = ranking::build_expansion_cache_key("alpha", 5, true, "llm", "model", 0.1_f32)
		.expect("Expected cache key.");

	assert_ne!(key_a, key_b);
}

#[test]
fn rerank_cache_key_changes_with_updated_at() {
	let ts_a = OffsetDateTime::from_unix_timestamp(1).expect("Valid timestamp.");
	let ts_b = OffsetDateTime::from_unix_timestamp(2).expect("Valid timestamp.");
	let chunk_id = Uuid::new_v4();
	let key_a = ranking::build_rerank_cache_key("q", "rerank", "model", &[(chunk_id, ts_a)])
		.expect("Expected cache key.");
	let key_b = ranking::build_rerank_cache_key("q", "rerank", "model", &[(chunk_id, ts_b)])
		.expect("Expected cache key.");

	assert_ne!(key_a, key_b);
}

#[test]
fn rerank_cache_payload_rejects_mismatched_counts() {
	let payload = RerankCachePayload {
		items: vec![RerankCacheItem {
			chunk_id: Uuid::new_v4(),
			updated_at: OffsetDateTime::from_unix_timestamp(1).expect("Valid timestamp."),
			score: 0.5,
		}],
	};
	let candidates = vec![RerankCacheCandidate {
		chunk_id: Uuid::new_v4(),
		updated_at: OffsetDateTime::from_unix_timestamp(1).expect("Valid timestamp."),
	}];

	assert!(ranking::build_cached_scores(&payload, &candidates).is_none());
}

#[test]
fn cache_key_prefix_is_stable() {
	let prefix = ranking::cache_key_prefix("abcd1234efgh5678");

	assert_eq!(prefix, "abcd1234efgh");
}

#[test]
fn lexical_overlap_ratio_is_deterministic_and_bounded() {
	let query_tokens = vec!["deploy".to_string(), "steps".to_string()];
	let ratio = ranking::lexical_overlap_ratio(&query_tokens, "Deploy steps for staging.", 128);

	assert!((ratio - 1.0).abs() < 1e-6, "Unexpected ratio: {ratio}");

	let ratio = ranking::lexical_overlap_ratio(&query_tokens, "Deploy only.", 128);

	assert!((ratio - 0.5).abs() < 1e-6, "Unexpected ratio: {ratio}");
	assert!((0.0..=1.0).contains(&ratio), "Ratio must be in [0, 1].");
}

#[test]
fn deterministic_ranking_terms_do_not_apply_when_disabled() {
	let mut cfg = parse_example_config();

	cfg.ranking.deterministic.enabled = false;
	cfg.ranking.deterministic.lexical.enabled = true;
	cfg.ranking.deterministic.hits.enabled = true;
	cfg.ranking.deterministic.decay.enabled = true;

	let now = OffsetDateTime::from_unix_timestamp(1_000_000).expect("Valid timestamp.");
	let note = NoteMeta {
		note_id: Uuid::new_v4(),
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
		hit_count: 8,
		last_hit_at: Some(now),
	};
	let chunk =
		ChunkMeta { chunk_id: Uuid::new_v4(), chunk_index: 0, start_offset: 0, end_offset: 10 };
	let item = ChunkSnippet {
		note,
		chunk,
		snippet: "deploy steps".to_string(),
		retrieval_rank: 1,
		retrieval_score: None,
	};
	let mut scored = ScoredChunk {
		item,
		final_score: 1.0,
		rerank_score: 0.5,
		rerank_rank: 1,
		rerank_norm: 1.0,
		retrieval_norm: 1.0,
		blend_retrieval_weight: 0.5,
		retrieval_term: 0.5,
		rerank_term: 0.5,
		tie_breaker_score: 0.0,
		scope_context_boost: 0.0,
		age_days: 30.0,
		importance: 0.1,
		deterministic_lexical_overlap_ratio: 0.0,
		deterministic_lexical_bonus: 0.0,
		deterministic_hit_count: 0,
		deterministic_last_hit_age_days: None,
		deterministic_hit_boost: 0.0,
		deterministic_decay_penalty: 0.0,
	};
	let terms = ranking::compute_deterministic_ranking_terms(
		&cfg,
		&ranking::tokenize_query(
			"deploy steps",
			cfg.ranking.deterministic.lexical.max_query_terms as usize,
		),
		scored.item.snippet.as_str(),
		scored.item.note.hit_count,
		scored.item.note.last_hit_at,
		scored.age_days,
		now,
	);

	scored.final_score += terms.lexical_bonus + terms.hit_boost + terms.decay_penalty;
	scored.deterministic_lexical_overlap_ratio = terms.lexical_overlap_ratio;
	scored.deterministic_lexical_bonus = terms.lexical_bonus;
	scored.deterministic_hit_count = terms.hit_count;
	scored.deterministic_last_hit_age_days = terms.last_hit_age_days;
	scored.deterministic_hit_boost = terms.hit_boost;
	scored.deterministic_decay_penalty = terms.decay_penalty;

	assert!((scored.final_score - 1.0).abs() < 1e-6, "Score must not change.");
	assert!((scored.deterministic_lexical_bonus - 0.0).abs() < 1e-6);
	assert!((scored.deterministic_hit_boost - 0.0).abs() < 1e-6);
	assert!((scored.deterministic_decay_penalty - 0.0).abs() < 1e-6);
}

#[test]
fn deterministic_ranking_terms_apply_and_are_bounded() {
	let mut cfg = parse_example_config();

	cfg.ranking.deterministic.enabled = true;
	cfg.ranking.deterministic.lexical.enabled = true;
	cfg.ranking.deterministic.hits.enabled = true;
	cfg.ranking.deterministic.decay.enabled = true;

	let now = OffsetDateTime::from_unix_timestamp(1_000_000).expect("Valid timestamp.");
	let note = NoteMeta {
		note_id: Uuid::new_v4(),
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
		hit_count: 8,
		last_hit_at: Some(now),
	};
	let chunk =
		ChunkMeta { chunk_id: Uuid::new_v4(), chunk_index: 0, start_offset: 0, end_offset: 10 };
	let item = ChunkSnippet {
		note,
		chunk,
		snippet: "deploy steps".to_string(),
		retrieval_rank: 1,
		retrieval_score: None,
	};
	let mut scored = ScoredChunk {
		item,
		final_score: 1.0,
		rerank_score: 0.5,
		rerank_rank: 1,
		rerank_norm: 1.0,
		retrieval_norm: 1.0,
		blend_retrieval_weight: 0.5,
		retrieval_term: 0.5,
		rerank_term: 0.5,
		tie_breaker_score: 0.0,
		scope_context_boost: 0.0,
		age_days: 30.0,
		importance: 0.1,
		deterministic_lexical_overlap_ratio: 0.0,
		deterministic_lexical_bonus: 0.0,
		deterministic_hit_count: 0,
		deterministic_last_hit_age_days: None,
		deterministic_hit_boost: 0.0,
		deterministic_decay_penalty: 0.0,
	};
	let terms = ranking::compute_deterministic_ranking_terms(
		&cfg,
		&ranking::tokenize_query(
			"deploy steps",
			cfg.ranking.deterministic.lexical.max_query_terms as usize,
		),
		scored.item.snippet.as_str(),
		scored.item.note.hit_count,
		scored.item.note.last_hit_at,
		scored.age_days,
		now,
	);

	scored.final_score += terms.lexical_bonus + terms.hit_boost + terms.decay_penalty;
	scored.deterministic_lexical_overlap_ratio = terms.lexical_overlap_ratio;
	scored.deterministic_lexical_bonus = terms.lexical_bonus;
	scored.deterministic_hit_count = terms.hit_count;
	scored.deterministic_last_hit_age_days = terms.last_hit_age_days;
	scored.deterministic_hit_boost = terms.hit_boost;
	scored.deterministic_decay_penalty = terms.decay_penalty;

	assert!(scored.final_score.is_finite(), "Score must be finite.");
	assert!((0.0..=1.0).contains(&scored.deterministic_lexical_overlap_ratio));
	assert!(scored.deterministic_lexical_bonus >= 0.0);
	assert!(scored.deterministic_hit_boost >= 0.0);
	assert!(scored.deterministic_decay_penalty <= 0.0);

	let expected_lex = cfg.ranking.deterministic.lexical.weight;

	assert!((scored.deterministic_lexical_bonus - expected_lex).abs() < 1e-6);

	let expected_hit = cfg.ranking.deterministic.hits.weight * 0.5;

	assert!((scored.deterministic_hit_boost - expected_hit).abs() < 1e-6);
}

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
