use std::collections::HashSet;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::app::{dataset, metrics, types::ExpectedKind};

#[test]
fn resolve_expected_mode_requires_exactly_one_definition() {
	let index = 0;
	let note_ids = vec![Uuid::new_v4()];
	let expected_keys = vec!["key-1".to_string()];
	let note_only = dataset::resolve_expected_mode(index, &note_ids, &[]);
	let key_only = dataset::resolve_expected_mode(index, &[], &expected_keys);
	let none = dataset::resolve_expected_mode(index, &[], &[]);
	let both = dataset::resolve_expected_mode(index, &note_ids, &expected_keys);

	assert!(matches!(note_only.unwrap(), ExpectedKind::NoteId));
	assert!(matches!(key_only.unwrap(), ExpectedKind::Key));
	assert!(none.is_err(), "Expected missing expectations to be rejected");
	assert!(both.is_err(), "Expected both expectation fields to be rejected");
}

#[test]
fn compute_metrics_for_keys_counts_first_hit_per_unique_key_and_ignores_missing_keys() {
	let expected: HashSet<String> =
		["alpha", "beta", "gamma"].into_iter().map(String::from).collect();
	let retrieved = vec![
		None,
		Some("alpha".to_string()),
		Some("alpha".to_string()),
		Some("gamma".to_string()),
		Some("missing".to_string()),
	];
	let metrics = metrics::compute_metrics_for_keys(&retrieved, &expected);
	let expected_dcg = 1.0 / (3.0_f64).log2() + 1.0 / (5.0_f64).log2();
	let expected_idcg = 1.0 + 1.0 / (3.0_f64).log2() + 1.0 / (4.0_f64).log2();

	assert_eq!(metrics.relevant_count, 2);
	assert!((metrics.precision_at_k - (2.0 / 5.0)).abs() < 1e-12);
	assert!((metrics.recall_at_k - (2.0 / 3.0)).abs() < 1e-12);
	assert!((metrics.rr - (1.0 / 2.0)).abs() < 1e-12);
	assert!((metrics.ndcg - (expected_dcg / expected_idcg)).abs() < 1e-12);
}

#[test]
fn retrieval_top_rank_retention_counts_unique_notes_and_retained_notes() {
	let now = OffsetDateTime::from_unix_timestamp(0).expect("Valid timestamp.");
	let note_a = Uuid::new_v4();
	let note_b = Uuid::new_v4();
	let note_c = Uuid::new_v4();
	let candidates = vec![
		elf_service::search::TraceReplayCandidate {
			note_id: note_a,
			chunk_id: Uuid::new_v4(),
			chunk_index: 0,
			snippet: "a".to_string(),
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
		elf_service::search::TraceReplayCandidate {
			note_id: note_a,
			chunk_id: Uuid::new_v4(),
			chunk_index: 1,
			snippet: "a".to_string(),
			retrieval_rank: 2,
			retrieval_score: None,
			rerank_score: 0.2,
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
		elf_service::search::TraceReplayCandidate {
			note_id: note_b,
			chunk_id: Uuid::new_v4(),
			chunk_index: 0,
			snippet: "b".to_string(),
			retrieval_rank: 3,
			retrieval_score: None,
			rerank_score: 0.3,
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
		elf_service::search::TraceReplayCandidate {
			note_id: note_c,
			chunk_id: Uuid::new_v4(),
			chunk_index: 0,
			snippet: "c".to_string(),
			retrieval_rank: 4,
			retrieval_score: None,
			rerank_score: 0.4,
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
	let note_ids = vec![note_a, note_c];
	let (total, retained, retention) =
		metrics::retrieval_top_rank_retention(&candidates, &note_ids, 3);

	assert_eq!(total, 2);
	assert_eq!(retained, 1);
	assert!((retention - 0.5).abs() < 1e-12, "Unexpected retention: {retention}");
}
