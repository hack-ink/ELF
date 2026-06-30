use serde::Serialize;
use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

use elf_service::search::TraceReplayItem;

#[derive(Debug, Serialize)]
pub(in crate::app) struct TraceCompareOutput {
	pub(super) policies: TraceComparePolicies,
	pub(super) summary: TraceCompareSummary,
	pub(super) traces: Vec<TraceCompareTrace>,
}

#[derive(Debug, Serialize)]
pub(super) struct TraceComparePolicies {
	pub(super) a: TraceComparePolicy,
	pub(super) b: TraceComparePolicy,
}

#[derive(Debug, Serialize)]
pub(super) struct TraceComparePolicy {
	pub(super) config_path: String,
	pub(super) policy_id: String,
}

#[derive(Debug, Serialize)]
pub(super) struct TraceCompareSummary {
	pub(super) trace_count: usize,
	pub(super) avg_positional_churn_at_k: f64,
	pub(super) avg_set_churn_at_k: f64,
	pub(super) avg_a_retrieval_top3_retention: f64,
	pub(super) avg_b_retrieval_top3_retention: f64,
	pub(super) avg_retrieval_top3_retention_delta: f64,
}

#[derive(Debug, Serialize)]
pub(super) struct TraceCompareTrace {
	pub(super) trace_id: Uuid,
	pub(super) query: String,
	pub(super) candidate_count: u32,
	pub(super) top_k: u32,
	pub(super) created_at: String,
	pub(super) a: TraceCompareVariant,
	pub(super) b: TraceCompareVariant,
	pub(super) churn: TraceCompareChurn,
	pub(super) guardrails: TraceCompareGuardrails,
	pub(super) stage_deltas: Vec<TraceCompareStageDelta>,
	pub(super) regression_attribution: TraceCompareRegressionAttribution,
}

#[derive(Debug, Serialize)]
pub(super) struct TraceCompareVariant {
	pub(super) policy_id: String,
	pub(super) items: Vec<TraceReplayItem>,
}

#[derive(Debug, Serialize)]
pub(super) struct TraceCompareChurn {
	pub(super) positional_churn_at_k: f64,
	pub(super) set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
pub(super) struct TraceCompareGuardrails {
	pub(super) retrieval_top3_total: usize,
	pub(super) a_retrieval_top3_retained: usize,
	pub(super) a_retrieval_top3_retention: f64,
	pub(super) b_retrieval_top3_retained: usize,
	pub(super) b_retrieval_top3_retention: f64,
	pub(super) retrieval_top3_retention_delta: f64,
}

#[derive(Debug, Serialize)]
pub(super) struct TraceCompareStageDelta {
	pub(super) stage_order: u32,
	pub(super) stage_name: String,
	pub(super) baseline_item_count: u32,
	pub(super) a_item_count: u32,
	pub(super) b_item_count: u32,
	pub(super) item_count_delta: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) baseline_stats: Option<Value>,
}

#[derive(Debug, Serialize)]
pub(super) struct TraceCompareRegressionAttribution {
	pub(super) primary_stage: String,
	pub(super) evidence: String,
}

#[derive(FromRow)]
pub(super) struct TraceCompareTraceRow {
	pub(super) trace_id: Uuid,
	pub(super) query: String,
	pub(super) candidate_count: i32,
	pub(super) top_k: i32,
	pub(super) created_at: OffsetDateTime,
}

#[derive(FromRow)]
pub(super) struct TraceCompareCandidateRow {
	pub(super) candidate_snapshot: Value,
	pub(super) note_id: Uuid,
	pub(super) chunk_id: Uuid,
	pub(super) chunk_index: i32,
	pub(super) snippet: String,
	pub(super) retrieval_rank: i32,
	pub(super) rerank_score: f32,
	pub(super) note_scope: String,
	pub(super) note_importance: f32,
	pub(super) note_updated_at: OffsetDateTime,
	pub(super) note_hit_count: i64,
	pub(super) note_last_hit_at: Option<OffsetDateTime>,
}

#[derive(FromRow)]
pub(super) struct TraceCompareStageRow {
	pub(super) stage_order: i32,
	pub(super) stage_name: String,
	pub(super) stage_payload: Value,
	pub(super) item_count: i64,
}

#[cfg(test)]
mod tests {
	use serde_json::json;
	use uuid::Uuid;

	use crate::app::trace_compare::types::{
		TraceCompareChurn, TraceCompareGuardrails, TraceCompareOutput, TraceComparePolicies,
		TraceComparePolicy, TraceCompareRegressionAttribution, TraceCompareStageDelta,
		TraceCompareSummary, TraceCompareTrace, TraceCompareVariant,
	};

	#[test]
	fn trace_compare_output_serializes_nested_report_contract() {
		let trace_id = Uuid::new_v4();
		let output = TraceCompareOutput {
			policies: TraceComparePolicies {
				a: TraceComparePolicy {
					config_path: "a.toml".to_string(),
					policy_id: "policy-a".to_string(),
				},
				b: TraceComparePolicy {
					config_path: "b.toml".to_string(),
					policy_id: "policy-b".to_string(),
				},
			},
			summary: TraceCompareSummary {
				trace_count: 1,
				avg_positional_churn_at_k: 0.25,
				avg_set_churn_at_k: 0.5,
				avg_a_retrieval_top3_retention: 1.0,
				avg_b_retrieval_top3_retention: 0.5,
				avg_retrieval_top3_retention_delta: -0.5,
			},
			traces: vec![TraceCompareTrace {
				trace_id,
				query: "find project context".to_string(),
				candidate_count: 7,
				top_k: 3,
				created_at: "2026-01-01T00:00:00Z".to_string(),
				a: TraceCompareVariant { policy_id: "policy-a".to_string(), items: Vec::new() },
				b: TraceCompareVariant { policy_id: "policy-b".to_string(), items: Vec::new() },
				churn: TraceCompareChurn { positional_churn_at_k: 0.25, set_churn_at_k: 0.5 },
				guardrails: TraceCompareGuardrails {
					retrieval_top3_total: 3,
					a_retrieval_top3_retained: 3,
					a_retrieval_top3_retention: 1.0,
					b_retrieval_top3_retained: 2,
					b_retrieval_top3_retention: 0.6667,
					retrieval_top3_retention_delta: -0.3333,
				},
				stage_deltas: vec![TraceCompareStageDelta {
					stage_order: 1,
					stage_name: "selection.final".to_string(),
					baseline_item_count: 3,
					a_item_count: 3,
					b_item_count: 2,
					item_count_delta: -1,
					baseline_stats: Some(json!({"selected": 3})),
				}],
				regression_attribution: TraceCompareRegressionAttribution {
					primary_stage: "selection.final".to_string(),
					evidence: "retention changed".to_string(),
				},
			}],
		};
		let value = serde_json::to_value(output).expect("Trace compare output serializes.");

		assert_eq!(value["policies"]["a"]["policy_id"], "policy-a");
		assert_eq!(value["summary"]["trace_count"], 1);
		assert_eq!(value["traces"][0]["trace_id"], trace_id.to_string());
		assert_eq!(value["traces"][0]["stage_deltas"][0]["baseline_stats"], json!({"selected": 3}));
		assert_eq!(
			value["traces"][0]["regression_attribution"]["primary_stage"],
			"selection.final"
		);
	}
}
