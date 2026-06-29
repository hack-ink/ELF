use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::app::SearchMode;
use elf_service::{RankingRequestOverride, SearchRequest, search::TraceReplayItem};

#[derive(Debug, Deserialize)]
pub(super) struct EvalDataset {
	pub(super) name: Option<String>,
	pub(super) defaults: Option<EvalDefaults>,
	pub(super) queries: Vec<EvalQuery>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct EvalDefaults {
	pub(super) tenant_id: Option<String>,
	pub(super) project_id: Option<String>,
	pub(super) agent_id: Option<String>,
	pub(super) read_profile: Option<String>,
	pub(super) top_k: Option<u32>,
	pub(super) candidate_k: Option<u32>,
	pub(super) ranking: Option<RankingRequestOverride>,
}

#[derive(Debug, Deserialize)]
pub(super) struct EvalQuery {
	pub(super) id: Option<String>,
	pub(super) query: String,
	pub(super) tenant_id: Option<String>,
	pub(super) project_id: Option<String>,
	pub(super) agent_id: Option<String>,
	pub(super) read_profile: Option<String>,
	pub(super) top_k: Option<u32>,
	pub(super) candidate_k: Option<u32>,
	#[serde(default)]
	pub(super) expected_note_ids: Vec<Uuid>,
	#[serde(default)]
	pub(super) expected_keys: Vec<String>,
	pub(super) ranking: Option<RankingRequestOverride>,
}

#[derive(Debug, Serialize)]
pub(super) struct EvalOutput {
	pub(super) dataset: EvalDatasetInfo,
	pub(super) settings: EvalSettings,
	pub(super) summary: EvalSummary,
	pub(super) queries: Vec<QueryReport>,
}

#[derive(Debug, Serialize)]
pub(super) struct EvalDatasetInfo {
	pub(super) name: String,
	pub(super) query_count: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct EvalSettings {
	pub(super) config_path: String,
	pub(super) search_mode: SearchMode,
	pub(super) candidate_k: u32,
	pub(super) top_k: u32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) runs_per_query: Option<u32>,
}

#[derive(Debug, Serialize)]
pub(super) struct EvalSummary {
	pub(super) avg_recall_at_k: f64,
	pub(super) avg_precision_at_k: f64,
	pub(super) mean_rr: f64,
	pub(super) mean_ndcg: f64,
	pub(super) latency_ms_p50: f64,
	pub(super) latency_ms_p95: f64,
	pub(super) avg_retrieved_summary_chars: f64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) stability: Option<StabilitySummary>,
}

#[derive(Debug, Serialize)]
pub(super) struct StabilitySummary {
	pub(super) runs_per_query: u32,
	pub(super) avg_positional_churn_at_k: f64,
	pub(super) avg_set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
pub(super) struct QueryReport {
	pub(super) id: String,
	pub(super) query: String,
	pub(super) trace_id: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) trace_ids: Option<Vec<Uuid>>,
	pub(super) expected_count: usize,
	pub(super) retrieved_count: usize,
	pub(super) relevant_count: usize,
	pub(super) recall_at_k: f64,
	pub(super) precision_at_k: f64,
	pub(super) rr: f64,
	pub(super) ndcg: f64,
	pub(super) latency_ms: f64,
	pub(super) expected_note_ids: Vec<Uuid>,
	pub(super) expected_keys: Vec<String>,
	pub(super) expected_kind: ExpectedKind,
	pub(super) retrieved_note_ids: Vec<Uuid>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub(super) retrieved_keys: Vec<Option<String>>,
	pub(super) retrieved_summary_chars: usize,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) stability: Option<QueryStability>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ExpectedKind {
	NoteId,
	Key,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct QueryStability {
	pub(super) runs_per_query: u32,
	pub(super) positional_churn_at_k: f64,
	pub(super) set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
pub(super) struct CompareOutput {
	pub(super) dataset: EvalDatasetInfo,
	pub(super) settings_a: EvalSettings,
	pub(super) settings_b: EvalSettings,
	pub(super) summary_a: EvalSummary,
	pub(super) summary_b: EvalSummary,
	pub(super) summary_delta: EvalSummaryDelta,
	pub(super) policy_stability: PolicyStabilitySummary,
	pub(super) queries: Vec<CompareQueryReport>,
}

#[derive(Debug, Serialize)]
pub(super) struct PolicyStabilitySummary {
	pub(super) k: u32,
	pub(super) avg_positional_churn_at_k: f64,
	pub(super) avg_set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
pub(super) struct EvalSummaryDelta {
	pub(super) avg_recall_at_k: f64,
	pub(super) avg_precision_at_k: f64,
	pub(super) mean_rr: f64,
	pub(super) mean_ndcg: f64,
	pub(super) latency_ms_p50: f64,
	pub(super) latency_ms_p95: f64,
	pub(super) avg_retrieved_summary_chars: f64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) stability: Option<StabilitySummaryDelta>,
}

#[derive(Debug, Serialize)]
pub(super) struct StabilitySummaryDelta {
	pub(super) avg_positional_churn_at_k: f64,
	pub(super) avg_set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
pub(super) struct CompareQueryReport {
	pub(super) id: String,
	pub(super) query: String,
	pub(super) expected_count: usize,
	pub(super) expected_note_ids: Vec<Uuid>,
	pub(super) a: QueryVariantReport,
	pub(super) b: QueryVariantReport,
	pub(super) delta: QueryVariantDelta,
	pub(super) policy_churn: PolicyChurn,
}

#[derive(Debug, Serialize)]
pub(super) struct PolicyChurn {
	pub(super) positional_churn_at_k: f64,
	pub(super) set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
pub(super) struct QueryVariantReport {
	pub(super) trace_id: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) trace_ids: Option<Vec<Uuid>>,
	pub(super) retrieved_count: usize,
	pub(super) relevant_count: usize,
	pub(super) recall_at_k: f64,
	pub(super) precision_at_k: f64,
	pub(super) rr: f64,
	pub(super) ndcg: f64,
	pub(super) latency_ms: f64,
	pub(super) retrieved_note_ids: Vec<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) stability: Option<QueryStability>,
}

#[derive(Debug, Serialize)]
pub(super) struct QueryVariantDelta {
	pub(super) retrieved_count: i64,
	pub(super) relevant_count: i64,
	pub(super) recall_at_k: f64,
	pub(super) precision_at_k: f64,
	pub(super) rr: f64,
	pub(super) ndcg: f64,
	pub(super) latency_ms: f64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) stability: Option<QueryStabilityDelta>,
}

#[derive(Debug, Serialize)]
pub(super) struct QueryStabilityDelta {
	pub(super) positional_churn_at_k: f64,
	pub(super) set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
pub(super) struct TraceCompareOutput {
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

pub(super) struct MergedQuery {
	pub(super) id: String,
	pub(super) query: String,
	pub(super) expected_note_ids: Vec<Uuid>,
	pub(super) expected_keys: Vec<String>,
	pub(super) expected_kind: ExpectedKind,
	pub(super) request: SearchRequest,
}

pub(super) struct Metrics {
	pub(super) recall_at_k: f64,
	pub(super) precision_at_k: f64,
	pub(super) rr: f64,
	pub(super) ndcg: f64,
	pub(super) relevant_count: usize,
}

pub(super) struct EvalRun {
	pub(super) dataset: EvalDatasetInfo,
	pub(super) settings: EvalSettings,
	pub(super) summary: EvalSummary,
	pub(super) queries: Vec<QueryReport>,
}

pub(super) fn default_eval_defaults() -> EvalDefaults {
	EvalDefaults {
		tenant_id: None,
		project_id: None,
		agent_id: None,
		read_profile: None,
		top_k: None,
		candidate_k: None,
		ranking: None,
	}
}
