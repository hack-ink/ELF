use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct GateReport {
	pub(super) config_path: String,
	pub(super) gate_path: String,
	pub(super) summary: GateSummary,
	pub(super) traces: Vec<TraceReport>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct GateSummary {
	pub(super) trace_count: usize,
	pub(super) breached_count: usize,
	pub(super) ok: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct TraceReport {
	pub(super) trace_id: Uuid,
	pub(super) query: String,
	pub(super) created_at: String,
	pub(super) top_k: u32,
	pub(super) retrieval_retention_rank: u32,
	pub(super) candidate_count: u32,
	pub(super) baseline_count: usize,
	pub(super) replay_count: usize,
	pub(super) churn: TraceChurn,
	pub(super) retention: TraceRetention,
	pub(super) breaches: Vec<GateBreach>,
	pub(super) ok: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct TraceChurn {
	pub(super) positional_churn_at_k: f64,
	pub(super) set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct TraceRetention {
	pub(super) retrieval_top_rank_total: usize,
	pub(super) baseline_retrieval_top_rank_retained: usize,
	pub(super) baseline_retrieval_top_rank_retention: f64,
	pub(super) replay_retrieval_top_rank_retained: usize,
	pub(super) replay_retrieval_top_rank_retention: f64,
	pub(super) retention_delta: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct GateBreach {
	pub(super) metric: String,
	pub(super) value: f64,
	pub(super) threshold: f64,
	pub(super) op: String,
}
