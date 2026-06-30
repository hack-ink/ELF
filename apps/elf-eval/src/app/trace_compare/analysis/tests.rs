use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::app::trace_compare::{
	analysis,
	types::{
		TraceCompareCandidateRow, TraceCompareChurn, TraceCompareGuardrails,
		TraceCompareStageDelta, TraceCompareStageRow,
	},
};
use elf_service::search::TraceReplayCandidate;

#[test]
fn stage_deltas_fallback_to_final_selection_when_baseline_stages_are_absent() {
	let deltas = analysis::build_trace_compare_stage_deltas(&[], 2, 4);

	assert_eq!(deltas.len(), 1);
	assert_eq!(deltas[0].stage_order, 1);
	assert_eq!(deltas[0].stage_name, "selection.final");
	assert_eq!(deltas[0].baseline_item_count, 0);
	assert_eq!(deltas[0].a_item_count, 2);
	assert_eq!(deltas[0].b_item_count, 4);
	assert_eq!(deltas[0].item_count_delta, 2);
	assert!(deltas[0].baseline_stats.is_none());
}

#[test]
fn stage_deltas_replace_final_selection_counts_and_preserve_stats() {
	let rows = vec![
		TraceCompareStageRow {
			stage_order: 1,
			stage_name: "recall.candidates".to_string(),
			stage_payload: json!({"stats": {"source": "baseline"}}),
			item_count: 7,
		},
		TraceCompareStageRow {
			stage_order: 2,
			stage_name: "selection.final".to_string(),
			stage_payload: json!({"stats": {"selected": true}}),
			item_count: 5,
		},
	];
	let deltas = analysis::build_trace_compare_stage_deltas(&rows, 3, 4);

	assert_eq!(deltas[0].baseline_item_count, 7);
	assert_eq!(deltas[0].a_item_count, 7);
	assert_eq!(deltas[0].b_item_count, 7);
	assert_eq!(deltas[0].baseline_stats, Some(json!({"source": "baseline"})));
	assert_eq!(deltas[1].baseline_item_count, 5);
	assert_eq!(deltas[1].a_item_count, 3);
	assert_eq!(deltas[1].b_item_count, 4);
	assert_eq!(deltas[1].item_count_delta, 1);
	assert_eq!(deltas[1].baseline_stats, Some(json!({"selected": true})));
}

#[test]
fn regression_attribution_prefers_retention_drop_with_recall_context() {
	let churn = TraceCompareChurn { positional_churn_at_k: 0.0, set_churn_at_k: 0.0 };
	let guardrails = TraceCompareGuardrails {
		retrieval_top3_total: 3,
		a_retrieval_top3_retained: 3,
		a_retrieval_top3_retention: 1.0,
		b_retrieval_top3_retained: 2,
		b_retrieval_top3_retention: 0.6667,
		retrieval_top3_retention_delta: -0.3333,
	};
	let stage_deltas = vec![TraceCompareStageDelta {
		stage_order: 1,
		stage_name: "recall.candidates".to_string(),
		baseline_item_count: 12,
		a_item_count: 12,
		b_item_count: 12,
		item_count_delta: 0,
		baseline_stats: None,
	}];
	let attribution =
		analysis::build_trace_compare_regression_attribution(&churn, &guardrails, &stage_deltas);

	assert_eq!(attribution.primary_stage, "selection.final");
	assert!(attribution.evidence.contains("dropped by -0.3333"));
	assert!(attribution.evidence.contains("recall baseline item_count=12"));
}

#[test]
fn regression_attribution_uses_rerank_when_churn_changes_without_retention_drop() {
	let churn = TraceCompareChurn { positional_churn_at_k: 0.5, set_churn_at_k: 0.25 };
	let guardrails = TraceCompareGuardrails {
		retrieval_top3_total: 3,
		a_retrieval_top3_retained: 2,
		a_retrieval_top3_retention: 0.6667,
		b_retrieval_top3_retained: 2,
		b_retrieval_top3_retention: 0.6667,
		retrieval_top3_retention_delta: 0.0,
	};
	let attribution =
		analysis::build_trace_compare_regression_attribution(&churn, &guardrails, &[]);

	assert_eq!(attribution.primary_stage, "rerank.score");
	assert!(attribution.evidence.contains("set_churn_at_k=0.2500"));
	assert!(attribution.evidence.contains("positional_churn_at_k=0.5000"));
}

#[test]
fn decode_candidates_falls_back_to_row_fields_when_snapshot_is_invalid() {
	let now = OffsetDateTime::from_unix_timestamp(0).expect("Valid timestamp.");
	let note_id = Uuid::new_v4();
	let chunk_id = Uuid::new_v4();
	let rows = vec![TraceCompareCandidateRow {
		candidate_snapshot: json!({"invalid": true}),
		note_id,
		chunk_id,
		chunk_index: 2,
		snippet: "candidate".to_string(),
		retrieval_rank: -1,
		rerank_score: 0.75,
		note_scope: "project_shared".to_string(),
		note_importance: 0.5,
		note_updated_at: now,
		note_hit_count: 9,
		note_last_hit_at: Some(now),
	}];
	let candidates = analysis::decode_trace_replay_candidates(rows);

	assert_eq!(candidates.len(), 1);
	assert_eq!(candidates[0].note_id, note_id);
	assert_eq!(candidates[0].chunk_id, chunk_id);
	assert_eq!(candidates[0].chunk_index, 2);
	assert_eq!(candidates[0].snippet, "candidate");
	assert_eq!(candidates[0].retrieval_rank, 0);
	assert_eq!(candidates[0].rerank_score, 0.75);
	assert_eq!(candidates[0].note_scope, "project_shared");
	assert_eq!(candidates[0].note_importance, 0.5);
	assert_eq!(candidates[0].note_updated_at, now);
	assert_eq!(candidates[0].note_hit_count, 9);
	assert_eq!(candidates[0].note_last_hit_at, Some(now));
	assert!(candidates[0].retrieval_score.is_none());
}

#[test]
fn decode_candidates_falls_back_when_valid_snapshot_has_nil_ids() {
	let now = OffsetDateTime::from_unix_timestamp(0).expect("Valid timestamp.");
	let note_id = Uuid::new_v4();
	let chunk_id = Uuid::new_v4();
	let snapshot = TraceReplayCandidate {
		note_id: Uuid::nil(),
		chunk_id: Uuid::new_v4(),
		chunk_index: 99,
		snippet: "snapshot".to_string(),
		retrieval_rank: 1,
		retrieval_score: Some(1.0),
		rerank_score: 1.0,
		note_scope: "snapshot_scope".to_string(),
		note_importance: 1.0,
		note_updated_at: now,
		note_hit_count: 1,
		note_last_hit_at: None,
		diversity_selected: None,
		diversity_selected_rank: None,
		diversity_selected_reason: None,
		diversity_skipped_reason: None,
		diversity_nearest_selected_note_id: None,
		diversity_similarity: None,
		diversity_mmr_score: None,
		diversity_missing_embedding: None,
	};
	let rows = vec![TraceCompareCandidateRow {
		candidate_snapshot: serde_json::to_value(snapshot).expect("Snapshot serializes."),
		note_id,
		chunk_id,
		chunk_index: 2,
		snippet: "candidate".to_string(),
		retrieval_rank: 3,
		rerank_score: 0.75,
		note_scope: "project_shared".to_string(),
		note_importance: 0.5,
		note_updated_at: now,
		note_hit_count: 9,
		note_last_hit_at: Some(now),
	}];
	let candidates = analysis::decode_trace_replay_candidates(rows);

	assert_eq!(candidates.len(), 1);
	assert_eq!(candidates[0].note_id, note_id);
	assert_eq!(candidates[0].chunk_id, chunk_id);
	assert_eq!(candidates[0].chunk_index, 2);
	assert_eq!(candidates[0].snippet, "candidate");
	assert_eq!(candidates[0].retrieval_rank, 3);
	assert_eq!(candidates[0].rerank_score, 0.75);
	assert_eq!(candidates[0].note_scope, "project_shared");
	assert_eq!(candidates[0].note_importance, 0.5);
	assert_eq!(candidates[0].note_updated_at, now);
	assert_eq!(candidates[0].note_hit_count, 9);
	assert_eq!(candidates[0].note_last_hit_at, Some(now));
	assert!(candidates[0].retrieval_score.is_none());
}
