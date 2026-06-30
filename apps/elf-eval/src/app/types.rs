use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::SearchMode;
use elf_service::{RankingRequestOverride, SearchRequest};

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
