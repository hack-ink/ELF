use crate::{
	EmbeddingMode, Serialize, Value,
	types::{
		backfill::BackfillReport,
		query::{QueryResult, QuerySummary},
		runtime::EmbeddingRuntimeReport,
	},
};

#[derive(Debug, Serialize)]
pub(crate) struct ResourceEnvelopeEvidence {
	pub(crate) elapsed_seconds: f64,
	pub(crate) max_elapsed_seconds: f64,
	pub(crate) rss_kb: Option<u64>,
	pub(crate) max_rss_kb: u64,
	pub(crate) postgres_database_bytes: Option<i64>,
	pub(crate) corpus_dir_bytes: u64,
	pub(crate) report_dir_bytes: Option<u64>,
	pub(crate) checkpoint_file_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CostProxyReport {
	pub(crate) schema: &'static str,
	pub(crate) scope: &'static str,
	pub(crate) embedding_mode: EmbeddingMode,
	pub(crate) estimated_input_chars: usize,
	pub(crate) estimated_input_tokens: usize,
	pub(crate) token_estimation: &'static str,
	pub(crate) configured_usd_per_1k_tokens: Option<f64>,
	pub(crate) estimated_usd: Option<f64>,
	pub(crate) document_count: usize,
	pub(crate) query_count: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct SoakConfig {
	pub(crate) target_seconds: u64,
	pub(crate) write_rounds: usize,
	pub(crate) probe_interval_millis: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct ElfBaselineReport {
	pub(crate) schema: &'static str,
	pub(crate) status: &'static str,
	pub(crate) retrieval_status: &'static str,
	pub(crate) reason: String,
	pub(crate) head: String,
	pub(crate) embedding: EmbeddingRuntimeReport,
	pub(crate) cost_proxy: CostProxyReport,
	pub(crate) backfill: BackfillReport,
	pub(crate) indexing: IndexingReport,
	pub(crate) summary: QuerySummary,
	pub(crate) check_summary: CheckSummary,
	pub(crate) checks: Vec<CheckResult>,
	pub(crate) queries: Vec<QueryResult>,
	pub(crate) ops_cases: Vec<OperationalCase>,
}

#[derive(Debug, Serialize)]
pub(crate) struct IndexingReport {
	pub(crate) note_count: usize,
	pub(crate) rebuild_rebuilt_count: u64,
	pub(crate) rebuild_missing_vector_count: u64,
	pub(crate) rebuild_error_count: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct OperationalCase {
	pub(crate) name: &'static str,
	pub(crate) default_status: &'static str,
	pub(crate) operator_status: &'static str,
	pub(crate) command: &'static str,
	pub(crate) evidence: &'static str,
	pub(crate) safety: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct CheckSummary {
	pub(crate) total: usize,
	pub(crate) pass: usize,
	pub(crate) fail: usize,
	pub(crate) wrong_result: usize,
	pub(crate) lifecycle_fail: usize,
	pub(crate) incomplete: usize,
	pub(crate) blocked: usize,
	pub(crate) not_encoded: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct CheckResult {
	pub(crate) name: &'static str,
	pub(crate) status: &'static str,
	pub(crate) reason: String,
	pub(crate) evidence: Value,
}
