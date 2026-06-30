use std::path::PathBuf;

use crate::search::{
	ChunkMeta, ChunkSnippet, NoteMeta, OffsetDateTime, ScoredChunk, Uuid, ranking,
};
use elf_config::Config;

fn parse_example_config() -> Config {
	let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
	let path = root_dir.join("elf.example.toml");

	elf_config::load(&path).expect("elf.example.toml must remain parseable and valid.")
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
