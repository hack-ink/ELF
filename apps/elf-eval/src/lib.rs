use std::{
	collections::{HashMap, HashSet},
	fs,
	path::{Path, PathBuf},
	time::Instant,
};

use clap::Parser;
use color_eyre::{Result, eyre};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use elf_config::Config;
use elf_service::{
	ElfService, RankingRequestOverride, SearchIndexItem, SearchIndexResponse, SearchRequest,
	search::TraceReplayItem,
};
use elf_storage::{db::Db, qdrant::QdrantStore};

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
pub struct Args {
	#[arg(long = "config-a", short = 'c', value_name = "FILE", visible_alias = "config")]
	pub config_a: PathBuf,
	#[arg(long = "config-b", value_name = "FILE")]
	pub config_b: Option<PathBuf>,
	#[arg(long, short = 'd', value_name = "FILE", required_unless_present = "trace_id")]
	pub dataset: Option<PathBuf>,
	#[arg(long, value_name = "N")]
	pub top_k: Option<u32>,
	#[arg(long, value_name = "N")]
	pub candidate_k: Option<u32>,
	#[arg(long, value_name = "N", default_value_t = 1)]
	pub runs_per_query: u32,
	#[arg(long = "trace-id", value_name = "UUID", num_args = 1..)]
	pub trace_id: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
struct EvalDataset {
	name: Option<String>,
	defaults: Option<EvalDefaults>,
	queries: Vec<EvalQuery>,
}

#[derive(Debug, Deserialize, Clone)]
struct EvalDefaults {
	tenant_id: Option<String>,
	project_id: Option<String>,
	agent_id: Option<String>,
	read_profile: Option<String>,
	top_k: Option<u32>,
	candidate_k: Option<u32>,
	ranking: Option<RankingRequestOverride>,
}

#[derive(Debug, Deserialize)]
struct EvalQuery {
	id: Option<String>,
	query: String,
	tenant_id: Option<String>,
	project_id: Option<String>,
	agent_id: Option<String>,
	read_profile: Option<String>,
	top_k: Option<u32>,
	candidate_k: Option<u32>,
	#[serde(default)]
	expected_note_ids: Vec<Uuid>,
	#[serde(default)]
	expected_keys: Vec<String>,
	ranking: Option<RankingRequestOverride>,
}

#[derive(Debug, Serialize)]
struct EvalOutput {
	dataset: EvalDatasetInfo,
	settings: EvalSettings,
	summary: EvalSummary,
	queries: Vec<QueryReport>,
}

#[derive(Debug, Serialize)]
struct EvalDatasetInfo {
	name: String,
	query_count: usize,
}

#[derive(Debug, Serialize)]
struct EvalSettings {
	config_path: String,
	candidate_k: u32,
	top_k: u32,
	#[serde(skip_serializing_if = "Option::is_none")]
	runs_per_query: Option<u32>,
}

#[derive(Debug, Serialize)]
struct EvalSummary {
	avg_recall_at_k: f64,
	avg_precision_at_k: f64,
	mean_rr: f64,
	mean_ndcg: f64,
	latency_ms_p50: f64,
	latency_ms_p95: f64,
	avg_retrieved_summary_chars: f64,
	#[serde(skip_serializing_if = "Option::is_none")]
	stability: Option<StabilitySummary>,
}

#[derive(Debug, Serialize)]
struct StabilitySummary {
	runs_per_query: u32,
	avg_positional_churn_at_k: f64,
	avg_set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
struct QueryReport {
	id: String,
	query: String,
	trace_id: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	trace_ids: Option<Vec<Uuid>>,
	expected_count: usize,
	retrieved_count: usize,
	relevant_count: usize,
	recall_at_k: f64,
	precision_at_k: f64,
	rr: f64,
	ndcg: f64,
	latency_ms: f64,
	expected_note_ids: Vec<Uuid>,
	expected_keys: Vec<String>,
	expected_kind: ExpectedKind,
	retrieved_note_ids: Vec<Uuid>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	retrieved_keys: Vec<Option<String>>,
	retrieved_summary_chars: usize,
	#[serde(skip_serializing_if = "Option::is_none")]
	stability: Option<QueryStability>,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ExpectedKind {
	NoteId,
	Key,
}

#[derive(Debug, Serialize, Clone, Copy)]
struct QueryStability {
	runs_per_query: u32,
	positional_churn_at_k: f64,
	set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
struct CompareOutput {
	dataset: EvalDatasetInfo,
	settings_a: EvalSettings,
	settings_b: EvalSettings,
	summary_a: EvalSummary,
	summary_b: EvalSummary,
	summary_delta: EvalSummaryDelta,
	policy_stability: PolicyStabilitySummary,
	queries: Vec<CompareQueryReport>,
}

#[derive(Debug, Serialize)]
struct PolicyStabilitySummary {
	k: u32,
	avg_positional_churn_at_k: f64,
	avg_set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
struct EvalSummaryDelta {
	avg_recall_at_k: f64,
	avg_precision_at_k: f64,
	mean_rr: f64,
	mean_ndcg: f64,
	latency_ms_p50: f64,
	latency_ms_p95: f64,
	avg_retrieved_summary_chars: f64,
	#[serde(skip_serializing_if = "Option::is_none")]
	stability: Option<StabilitySummaryDelta>,
}

#[derive(Debug, Serialize)]
struct StabilitySummaryDelta {
	avg_positional_churn_at_k: f64,
	avg_set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
struct CompareQueryReport {
	id: String,
	query: String,
	expected_count: usize,
	expected_note_ids: Vec<Uuid>,
	a: QueryVariantReport,
	b: QueryVariantReport,
	delta: QueryVariantDelta,
	policy_churn: PolicyChurn,
}

#[derive(Debug, Serialize)]
struct PolicyChurn {
	positional_churn_at_k: f64,
	set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
struct QueryVariantReport {
	trace_id: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	trace_ids: Option<Vec<Uuid>>,
	retrieved_count: usize,
	relevant_count: usize,
	recall_at_k: f64,
	precision_at_k: f64,
	rr: f64,
	ndcg: f64,
	latency_ms: f64,
	retrieved_note_ids: Vec<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	stability: Option<QueryStability>,
}

#[derive(Debug, Serialize)]
struct QueryVariantDelta {
	retrieved_count: i64,
	relevant_count: i64,
	recall_at_k: f64,
	precision_at_k: f64,
	rr: f64,
	ndcg: f64,
	latency_ms: f64,
	#[serde(skip_serializing_if = "Option::is_none")]
	stability: Option<QueryStabilityDelta>,
}

#[derive(Debug, Serialize)]
struct QueryStabilityDelta {
	positional_churn_at_k: f64,
	set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
struct TraceCompareOutput {
	policies: TraceComparePolicies,
	summary: TraceCompareSummary,
	traces: Vec<TraceCompareTrace>,
}

#[derive(Debug, Serialize)]
struct TraceComparePolicies {
	a: TraceComparePolicy,
	b: TraceComparePolicy,
}

#[derive(Debug, Serialize)]
struct TraceComparePolicy {
	config_path: String,
	policy_id: String,
}

#[derive(Debug, Serialize)]
struct TraceCompareSummary {
	trace_count: usize,
	avg_positional_churn_at_k: f64,
	avg_set_churn_at_k: f64,
	avg_a_retrieval_top3_retention: f64,
	avg_b_retrieval_top3_retention: f64,
	avg_retrieval_top3_retention_delta: f64,
}

#[derive(Debug, Serialize)]
struct TraceCompareTrace {
	trace_id: Uuid,
	query: String,
	candidate_count: u32,
	top_k: u32,
	created_at: String,
	a: TraceCompareVariant,
	b: TraceCompareVariant,
	churn: TraceCompareChurn,
	guardrails: TraceCompareGuardrails,
	stage_deltas: Vec<TraceCompareStageDelta>,
	regression_attribution: TraceCompareRegressionAttribution,
}

#[derive(Debug, Serialize)]
struct TraceCompareVariant {
	policy_id: String,
	items: Vec<TraceReplayItem>,
}

#[derive(Debug, Serialize)]
struct TraceCompareChurn {
	positional_churn_at_k: f64,
	set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
struct TraceCompareGuardrails {
	retrieval_top3_total: usize,
	a_retrieval_top3_retained: usize,
	a_retrieval_top3_retention: f64,
	b_retrieval_top3_retained: usize,
	b_retrieval_top3_retention: f64,
	retrieval_top3_retention_delta: f64,
}

#[derive(Debug, Serialize)]
struct TraceCompareStageDelta {
	stage_order: u32,
	stage_name: String,
	baseline_item_count: u32,
	a_item_count: u32,
	b_item_count: u32,
	item_count_delta: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	baseline_stats: Option<Value>,
}

#[derive(Debug, Serialize)]
struct TraceCompareRegressionAttribution {
	primary_stage: String,
	evidence: String,
}

#[derive(FromRow)]
struct TraceCompareTraceRow {
	trace_id: Uuid,
	query: String,
	candidate_count: i32,
	top_k: i32,
	created_at: OffsetDateTime,
}

#[derive(FromRow)]
struct TraceCompareCandidateRow {
	candidate_snapshot: Value,
	note_id: Uuid,
	chunk_id: Uuid,
	chunk_index: i32,
	snippet: String,
	retrieval_rank: i32,
	rerank_score: f32,
	note_scope: String,
	note_importance: f32,
	note_updated_at: OffsetDateTime,
	note_hit_count: i64,
	note_last_hit_at: Option<OffsetDateTime>,
}

#[derive(FromRow)]
struct TraceCompareStageRow {
	stage_order: i32,
	stage_name: String,
	stage_payload: Value,
	item_count: i64,
}

struct MergedQuery {
	id: String,
	query: String,
	expected_note_ids: Vec<Uuid>,
	expected_keys: Vec<String>,
	expected_kind: ExpectedKind,
	request: SearchRequest,
}

struct Metrics {
	recall_at_k: f64,
	precision_at_k: f64,
	rr: f64,
	ndcg: f64,
	relevant_count: usize,
}

struct EvalRun {
	dataset: EvalDatasetInfo,
	settings: EvalSettings,
	summary: EvalSummary,
	queries: Vec<QueryReport>,
}

pub async fn run(args: Args) -> Result<()> {
	let config_a = elf_config::load(&args.config_a)?;
	let filter = EnvFilter::new(config_a.service.log_level.clone());

	tracing_subscriber::fmt().with_env_filter(filter).init();

	if !args.trace_id.is_empty() {
		let Some(config_b_path) = &args.config_b else {
			return Err(eyre::eyre!("Trace compare mode requires --config-b."));
		};
		let config_b = elf_config::load(config_b_path)?;
		let output = trace_compare(
			args.config_a.as_path(),
			config_a,
			config_b_path.as_path(),
			config_b,
			&args,
		)
		.await?;
		let json = serde_json::to_string_pretty(&output)?;

		println!("{json}");

		return Ok(());
	}

	let dataset_path =
		args.dataset.as_ref().ok_or_else(|| eyre::eyre!("--dataset is required."))?;
	let dataset = load_dataset(dataset_path.as_path())?;
	let run_a = eval_config(args.config_a.as_path(), config_a, &dataset, &args).await?;

	if let Some(config_b_path) = &args.config_b {
		let config_b = elf_config::load(config_b_path)?;
		let run_b = eval_config(config_b_path.as_path(), config_b, &dataset, &args).await?;
		let k = run_a.settings.top_k.min(run_b.settings.top_k).max(1);
		let (queries, policy_stability) = build_compare_queries(&run_a.queries, &run_b.queries, k);
		let summary_delta = diff_summary(&run_a.summary, &run_b.summary);
		let output = CompareOutput {
			dataset: run_a.dataset,
			settings_a: run_a.settings,
			settings_b: run_b.settings,
			summary_a: run_a.summary,
			summary_b: run_b.summary,
			summary_delta,
			policy_stability,
			queries,
		};
		let json = serde_json::to_string_pretty(&output)?;

		println!("{json}");

		return Ok(());
	}

	let output = EvalOutput {
		dataset: run_a.dataset,
		settings: run_a.settings,
		summary: run_a.summary,
		queries: run_a.queries,
	};
	let json = serde_json::to_string_pretty(&output)?;

	println!("{json}");

	Ok(())
}

fn retrieval_top_rank_retention(
	candidates: &[elf_service::search::TraceReplayCandidate],
	note_ids: &[Uuid],
	max_retrieval_rank: u32,
) -> (usize, usize, f64) {
	let mut top_notes = HashSet::new();

	for candidate in candidates {
		if candidate.retrieval_rank == 0 || candidate.retrieval_rank > max_retrieval_rank {
			continue;
		}

		top_notes.insert(candidate.note_id);
	}

	let total = top_notes.len();

	if total == 0 {
		return (0, 0, 0.0);
	}

	let out_set: HashSet<Uuid> = note_ids.iter().copied().collect();
	let retained = top_notes.intersection(&out_set).count();
	let retention = retained as f64 / total as f64;

	(total, retained, retention)
}

fn load_dataset(path: &Path) -> Result<EvalDataset> {
	let raw = fs::read_to_string(path)?;
	let dataset: EvalDataset = serde_json::from_str(&raw)?;

	if dataset.queries.is_empty() {
		return Err(eyre::eyre!("Dataset must include at least one query."));
	}

	Ok(dataset)
}

fn churn_against_baseline_at_k(baseline: &[Uuid], other: &[Uuid], k: usize) -> (f64, f64) {
	let k = k.max(1);
	let mut positional_diff = 0_usize;

	for idx in 0..k {
		let a = baseline.get(idx);
		let b = other.get(idx);

		if a != b {
			positional_diff += 1;
		}
	}

	let positional_churn = positional_diff as f64 / k as f64;
	let base_set: HashSet<Uuid> = baseline.iter().take(k).copied().collect();
	let other_set: HashSet<Uuid> = other.iter().take(k).copied().collect();
	let overlap = base_set.intersection(&other_set).count();
	let set_churn = 1.0 - (overlap as f64 / k as f64);

	(positional_churn, set_churn)
}

fn diff_summary(a: &EvalSummary, b: &EvalSummary) -> EvalSummaryDelta {
	EvalSummaryDelta {
		avg_recall_at_k: b.avg_recall_at_k - a.avg_recall_at_k,
		avg_precision_at_k: b.avg_precision_at_k - a.avg_precision_at_k,
		mean_rr: b.mean_rr - a.mean_rr,
		mean_ndcg: b.mean_ndcg - a.mean_ndcg,
		latency_ms_p50: b.latency_ms_p50 - a.latency_ms_p50,
		latency_ms_p95: b.latency_ms_p95 - a.latency_ms_p95,
		avg_retrieved_summary_chars: b.avg_retrieved_summary_chars - a.avg_retrieved_summary_chars,
		stability: match (&a.stability, &b.stability) {
			(Some(sa), Some(sb)) => Some(StabilitySummaryDelta {
				avg_positional_churn_at_k: sb.avg_positional_churn_at_k
					- sa.avg_positional_churn_at_k,
				avg_set_churn_at_k: sb.avg_set_churn_at_k - sa.avg_set_churn_at_k,
			}),
			_ => None,
		},
	}
}

fn build_compare_queries(
	a: &[QueryReport],
	b: &[QueryReport],
	k: u32,
) -> (Vec<CompareQueryReport>, PolicyStabilitySummary) {
	let k_usize = k.max(1) as usize;
	let mut positional_sum = 0.0_f64;
	let mut set_sum = 0.0_f64;
	let queries: Vec<CompareQueryReport> = a
		.iter()
		.zip(b.iter())
		.map(|(qa, qb)| {
			let delta_stability = match (qa.stability, qb.stability) {
				(Some(sa), Some(sb)) => Some(QueryStabilityDelta {
					positional_churn_at_k: sb.positional_churn_at_k - sa.positional_churn_at_k,
					set_churn_at_k: sb.set_churn_at_k - sa.set_churn_at_k,
				}),
				_ => None,
			};
			let (positional_churn_at_k, set_churn_at_k) = churn_against_baseline_at_k(
				&qa.retrieved_note_ids,
				&qb.retrieved_note_ids,
				k_usize,
			);

			positional_sum += positional_churn_at_k;
			set_sum += set_churn_at_k;

			CompareQueryReport {
				id: qa.id.clone(),
				query: qa.query.clone(),
				expected_count: qa.expected_count,
				expected_note_ids: qa.expected_note_ids.clone(),
				a: QueryVariantReport {
					trace_id: qa.trace_id,
					trace_ids: qa.trace_ids.clone(),
					retrieved_count: qa.retrieved_count,
					relevant_count: qa.relevant_count,
					recall_at_k: qa.recall_at_k,
					precision_at_k: qa.precision_at_k,
					rr: qa.rr,
					ndcg: qa.ndcg,
					latency_ms: qa.latency_ms,
					retrieved_note_ids: qa.retrieved_note_ids.clone(),
					stability: qa.stability,
				},
				b: QueryVariantReport {
					trace_id: qb.trace_id,
					trace_ids: qb.trace_ids.clone(),
					retrieved_count: qb.retrieved_count,
					relevant_count: qb.relevant_count,
					recall_at_k: qb.recall_at_k,
					precision_at_k: qb.precision_at_k,
					rr: qb.rr,
					ndcg: qb.ndcg,
					latency_ms: qb.latency_ms,
					retrieved_note_ids: qb.retrieved_note_ids.clone(),
					stability: qb.stability,
				},
				delta: QueryVariantDelta {
					retrieved_count: qb.retrieved_count as i64 - qa.retrieved_count as i64,
					relevant_count: qb.relevant_count as i64 - qa.relevant_count as i64,
					recall_at_k: qb.recall_at_k - qa.recall_at_k,
					precision_at_k: qb.precision_at_k - qa.precision_at_k,
					rr: qb.rr - qa.rr,
					ndcg: qb.ndcg - qa.ndcg,
					latency_ms: qb.latency_ms - qa.latency_ms,
					stability: delta_stability,
				},
				policy_churn: PolicyChurn { positional_churn_at_k, set_churn_at_k },
			}
		})
		.collect();
	let count = queries.len().max(1) as f64;
	let summary = PolicyStabilitySummary {
		k,
		avg_positional_churn_at_k: positional_sum / count,
		avg_set_churn_at_k: set_sum / count,
	};

	(queries, summary)
}

fn merge_query(
	defaults: &EvalDefaults,
	query: &EvalQuery,
	args: &Args,
	cfg: &Config,
	index: usize,
) -> Result<MergedQuery> {
	let expected_kind =
		resolve_expected_mode(index, &query.expected_note_ids, &query.expected_keys)?;
	let tenant_id = query
		.tenant_id
		.clone()
		.or_else(|| defaults.tenant_id.clone())
		.ok_or_else(|| eyre::eyre!("tenant_id is required for query at index {index}."))?;
	let project_id = query
		.project_id
		.clone()
		.or_else(|| defaults.project_id.clone())
		.ok_or_else(|| eyre::eyre!("project_id is required for query at index {index}."))?;
	let agent_id = query
		.agent_id
		.clone()
		.or_else(|| defaults.agent_id.clone())
		.ok_or_else(|| eyre::eyre!("agent_id is required for query at index {index}."))?;
	let read_profile = query
		.read_profile
		.clone()
		.or_else(|| defaults.read_profile.clone())
		.ok_or_else(|| eyre::eyre!("read_profile is required for query at index {index}."))?;
	let top_k = args.top_k.or(query.top_k).or(defaults.top_k).unwrap_or(cfg.memory.top_k).max(1);
	let candidate_k = args
		.candidate_k
		.or(query.candidate_k)
		.or(defaults.candidate_k)
		.unwrap_or(cfg.memory.candidate_k)
		.max(top_k);
	let id = query.id.clone().unwrap_or_else(|| format!("query-{index}"));
	let ranking = query.ranking.clone().or_else(|| defaults.ranking.clone());

	Ok(MergedQuery {
		id,
		query: query.query.clone(),
		expected_note_ids: query.expected_note_ids.clone(),
		expected_keys: query.expected_keys.clone(),
		expected_kind,
		request: SearchRequest {
			tenant_id,
			project_id,
			agent_id,
			token_id: None,
			read_profile,
			payload_level: Default::default(),
			query: query.query.clone(),
			top_k: Some(top_k),
			candidate_k: Some(candidate_k),
			filter: None,
			record_hits: Some(false),
			ranking,
		},
	})
}

fn resolve_expected_mode(index: usize, note_ids: &[Uuid], keys: &[String]) -> Result<ExpectedKind> {
	let has_note_ids = !note_ids.is_empty();
	let has_keys = !keys.is_empty();

	match (has_note_ids, has_keys) {
		(true, false) => Ok(ExpectedKind::NoteId),
		(false, true) => Ok(ExpectedKind::Key),
		(true, true) => Err(eyre::eyre!(
			"Query at index {index} must define exactly one expectation mode: expected_note_ids or expected_keys."
		)),
		(false, false) => Err(eyre::eyre!(
			"Query at index {index} must include at least one expected_note_ids or expected_keys."
		)),
	}
}

fn unique_items(items: &[SearchIndexItem]) -> Vec<SearchIndexItem> {
	let mut seen = HashSet::new();
	let mut out = Vec::new();

	for item in items {
		if seen.insert(item.note_id) {
			out.push(item.clone());
		}
	}

	out
}

fn compute_metrics(retrieved: &[Uuid], expected: &HashSet<Uuid>) -> Metrics {
	let expected_count = expected.len();
	let mut relevant_count = 0_usize;
	let mut dcg = 0.0_f64;
	let mut rr = 0.0_f64;
	let mut first_hit: Option<usize> = None;

	for (idx, id) in retrieved.iter().enumerate() {
		if expected.contains(id) {
			relevant_count += 1;

			let rank = idx + 1;
			let denom = (rank as f64 + 1.0).log2();

			dcg += 1.0 / denom;

			if first_hit.is_none() {
				first_hit = Some(rank);
			}
		}
	}

	if let Some(rank) = first_hit {
		rr = 1.0 / rank as f64;
	}

	let ideal_hits = expected_count.min(retrieved.len());
	let mut idcg = 0.0_f64;

	for idx in 0..ideal_hits {
		let rank = idx + 1;
		let denom = (rank as f64 + 1.0).log2();

		idcg += 1.0 / denom;
	}

	let ndcg = if idcg > 0.0 { dcg / idcg } else { 0.0 };
	let precision_at_k =
		if retrieved.is_empty() { 0.0 } else { relevant_count as f64 / retrieved.len() as f64 };
	let recall_at_k =
		if expected_count == 0 { 0.0 } else { relevant_count as f64 / expected_count as f64 };

	Metrics { recall_at_k, precision_at_k, rr, ndcg, relevant_count }
}

fn compute_metrics_for_keys(retrieved: &[Option<String>], expected: &HashSet<String>) -> Metrics {
	let expected_count = expected.len();
	let mut matched: HashSet<String> = HashSet::new();
	let mut relevant_count = 0_usize;
	let mut dcg = 0.0_f64;
	let mut rr = 0.0_f64;
	let mut first_hit: Option<usize> = None;

	for (idx, maybe_key) in retrieved.iter().enumerate() {
		let Some(key) = maybe_key else {
			continue;
		};

		if expected.contains(key) && !matched.contains(key) {
			matched.insert(key.clone());

			relevant_count += 1;

			let rank = idx + 1;
			let denom = (rank as f64 + 1.0).log2();

			dcg += 1.0 / denom;

			if first_hit.is_none() {
				first_hit = Some(rank);
			}
		}
	}

	if let Some(rank) = first_hit {
		rr = 1.0 / rank as f64;
	}

	let ideal_hits = expected_count.min(retrieved.len());
	let mut idcg = 0.0_f64;

	for idx in 0..ideal_hits {
		let rank = idx + 1;
		let denom = (rank as f64 + 1.0).log2();

		idcg += 1.0 / denom;
	}

	let ndcg = if idcg > 0.0 { dcg / idcg } else { 0.0 };
	let precision_at_k =
		if retrieved.is_empty() { 0.0 } else { relevant_count as f64 / retrieved.len() as f64 };
	let recall_at_k =
		if expected_count == 0 { 0.0 } else { relevant_count as f64 / expected_count as f64 };

	Metrics { recall_at_k, precision_at_k, rr, ndcg, relevant_count }
}

fn compute_metrics_for_query(
	merged: &MergedQuery,
	retrieved_note_ids: &[Uuid],
	retrieved_keys: &[Option<String>],
) -> (Metrics, usize) {
	match merged.expected_kind {
		ExpectedKind::NoteId => {
			let expected: HashSet<Uuid> = merged.expected_note_ids.iter().copied().collect();
			let expected_count = expected.len();

			(compute_metrics(retrieved_note_ids, &expected), expected_count)
		},
		ExpectedKind::Key => {
			let expected: HashSet<String> = merged.expected_keys.iter().cloned().collect();
			let expected_count = expected.len();

			(compute_metrics_for_keys(retrieved_keys, &expected), expected_count)
		},
	}
}

fn summarize(reports: &[QueryReport], latencies_ms: &[f64]) -> EvalSummary {
	let count = reports.len().max(1) as f64;
	let avg_recall_at_k = reports.iter().map(|r| r.recall_at_k).sum::<f64>() / count;
	let avg_precision_at_k = reports.iter().map(|r| r.precision_at_k).sum::<f64>() / count;
	let mean_rr = reports.iter().map(|r| r.rr).sum::<f64>() / count;
	let mean_ndcg = reports.iter().map(|r| r.ndcg).sum::<f64>() / count;
	let avg_retrieved_summary_chars =
		reports.iter().map(|r| r.retrieved_summary_chars as f64).sum::<f64>() / count;
	let mut sorted = latencies_ms.to_vec();

	sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

	let p50 = percentile(&sorted, 0.50);
	let p95 = percentile(&sorted, 0.95);

	EvalSummary {
		avg_recall_at_k,
		avg_precision_at_k,
		mean_rr,
		mean_ndcg,
		latency_ms_p50: p50,
		latency_ms_p95: p95,
		avg_retrieved_summary_chars,
		stability: None,
	}
}

fn percentile(values: &[f64], percentile: f64) -> f64 {
	if values.is_empty() {
		return 0.0;
	}

	let clamped = percentile.clamp(0.0, 1.0);
	let pos = clamped * (values.len() as f64 - 1.0);
	let lower = pos.floor() as usize;
	let upper = pos.ceil() as usize;

	if lower == upper {
		values[lower]
	} else {
		let weight = pos - lower as f64;

		values[lower] * (1.0 - weight) + values[upper] * weight
	}
}

fn decode_trace_replay_candidates(
	rows: Vec<TraceCompareCandidateRow>,
) -> Vec<elf_service::search::TraceReplayCandidate> {
	rows.into_iter()
		.map(|row| {
			let decoded = serde_json::from_value::<elf_service::search::TraceReplayCandidate>(
				row.candidate_snapshot.clone(),
			)
			.ok()
			.filter(|value| value.note_id != Uuid::nil() && value.chunk_id != Uuid::nil());

			decoded.unwrap_or_else(|| elf_service::search::TraceReplayCandidate {
				note_id: row.note_id,
				chunk_id: row.chunk_id,
				chunk_index: row.chunk_index,
				snippet: row.snippet,
				retrieval_rank: u32::try_from(row.retrieval_rank).unwrap_or(0),
				rerank_score: row.rerank_score,
				note_scope: row.note_scope,
				note_importance: row.note_importance,
				note_updated_at: row.note_updated_at,
				note_hit_count: row.note_hit_count,
				note_last_hit_at: row.note_last_hit_at,
				diversity_selected: None,
				diversity_selected_rank: None,
				diversity_selected_reason: None,
				diversity_skipped_reason: None,
				diversity_nearest_selected_note_id: None,
				diversity_similarity: None,
				diversity_mmr_score: None,
				diversity_missing_embedding: None,
			})
		})
		.collect()
}

fn build_trace_compare_stage_deltas(
	stage_rows: &[TraceCompareStageRow],
	a_selected_count: u32,
	b_selected_count: u32,
) -> Vec<TraceCompareStageDelta> {
	if stage_rows.is_empty() {
		return vec![TraceCompareStageDelta {
			stage_order: 1,
			stage_name: "selection.final".to_string(),
			baseline_item_count: 0,
			a_item_count: a_selected_count,
			b_item_count: b_selected_count,
			item_count_delta: b_selected_count as i64 - a_selected_count as i64,
			baseline_stats: None,
		}];
	}

	let mut out = Vec::with_capacity(stage_rows.len());

	for row in stage_rows {
		let baseline_item_count = row.item_count.max(0) as u32;
		let (a_item_count, b_item_count) = if row.stage_name == "selection.final" {
			(a_selected_count, b_selected_count)
		} else {
			(baseline_item_count, baseline_item_count)
		};
		let baseline_stats = row.stage_payload.get("stats").cloned();

		out.push(TraceCompareStageDelta {
			stage_order: row.stage_order.max(0) as u32,
			stage_name: row.stage_name.clone(),
			baseline_item_count,
			a_item_count,
			b_item_count,
			item_count_delta: b_item_count as i64 - a_item_count as i64,
			baseline_stats,
		});
	}

	out
}

fn build_trace_compare_regression_attribution(
	churn: &TraceCompareChurn,
	guardrails: &TraceCompareGuardrails,
	stage_deltas: &[TraceCompareStageDelta],
) -> TraceCompareRegressionAttribution {
	let stage_by_name: HashMap<&str, &TraceCompareStageDelta> =
		stage_deltas.iter().map(|stage| (stage.stage_name.as_str(), stage)).collect();

	if guardrails.retrieval_top3_retention_delta < 0.0 {
		let recall_count = stage_by_name
			.get("recall.candidates")
			.map(|stage| stage.baseline_item_count)
			.unwrap_or(0);

		return TraceCompareRegressionAttribution {
			primary_stage: "selection.final".to_string(),
			evidence: format!(
				"retrieval_top3_retention dropped by {:.4} (a={:.4}, b={:.4}); recall baseline item_count={recall_count}",
				guardrails.retrieval_top3_retention_delta,
				guardrails.a_retrieval_top3_retention,
				guardrails.b_retrieval_top3_retention
			),
		};
	}
	if churn.set_churn_at_k > 0.0 || churn.positional_churn_at_k > 0.0 {
		return TraceCompareRegressionAttribution {
			primary_stage: "rerank.score".to_string(),
			evidence: format!(
				"top-k churn changed without retrieval-top3 regression (set_churn_at_k={:.4}, positional_churn_at_k={:.4})",
				churn.set_churn_at_k, churn.positional_churn_at_k
			),
		};
	}

	TraceCompareRegressionAttribution {
		primary_stage: "not_applicable".to_string(),
		evidence: "No regression signal detected.".to_string(),
	}
}

async fn trace_compare(
	config_a_path: &Path,
	config_a: Config,
	config_b_path: &Path,
	config_b: Config,
	args: &Args,
) -> Result<TraceCompareOutput> {
	let policy_id_a = elf_service::search::ranking_policy_id(&config_a, None)
		.map_err(|err| eyre::eyre!("{err}"))?;
	let policy_id_b = elf_service::search::ranking_policy_id(&config_b, None)
		.map_err(|err| eyre::eyre!("{err}"))?;
	let db = Db::connect(&config_a.storage.postgres).await?;

	db.ensure_schema(config_a.storage.qdrant.vector_dim).await?;

	let mut traces = Vec::with_capacity(args.trace_id.len());
	let mut positional_sum = 0.0_f64;
	let mut set_sum = 0.0_f64;
	let mut top3_retention_a_sum = 0.0_f64;
	let mut top3_retention_b_sum = 0.0_f64;

	for trace_id in &args.trace_id {
		let trace = compare_trace_id(
			&db,
			&config_a,
			&config_b,
			policy_id_a.as_str(),
			policy_id_b.as_str(),
			trace_id,
			args,
		)
		.await?;

		positional_sum += trace.churn.positional_churn_at_k;
		set_sum += trace.churn.set_churn_at_k;
		top3_retention_a_sum += trace.guardrails.a_retrieval_top3_retention;
		top3_retention_b_sum += trace.guardrails.b_retrieval_top3_retention;

		traces.push(trace);
	}

	let count = traces.len().max(1) as f64;
	let summary = TraceCompareSummary {
		trace_count: traces.len(),
		avg_positional_churn_at_k: positional_sum / count,
		avg_set_churn_at_k: set_sum / count,
		avg_a_retrieval_top3_retention: top3_retention_a_sum / count,
		avg_b_retrieval_top3_retention: top3_retention_b_sum / count,
		avg_retrieval_top3_retention_delta: (top3_retention_b_sum - top3_retention_a_sum) / count,
	};

	Ok(TraceCompareOutput {
		policies: TraceComparePolicies {
			a: TraceComparePolicy {
				config_path: config_a_path.display().to_string(),
				policy_id: policy_id_a,
			},
			b: TraceComparePolicy {
				config_path: config_b_path.display().to_string(),
				policy_id: policy_id_b,
			},
		},
		summary,
		traces,
	})
}

async fn compare_trace_id(
	db: &Db,
	config_a: &Config,
	config_b: &Config,
	policy_id_a: &str,
	policy_id_b: &str,
	trace_id: &Uuid,
	args: &Args,
) -> Result<TraceCompareTrace> {
	let trace_row = fetch_trace_compare_trace_row(db, trace_id).await?;
	let candidate_rows = fetch_trace_compare_candidate_rows(db, trace_id).await?;
	let stage_rows = fetch_trace_compare_stage_rows(db, trace_id).await?;
	let context = elf_service::search::TraceReplayContext {
		trace_id: trace_row.trace_id,
		query: trace_row.query.clone(),
		candidate_count: u32::try_from(trace_row.candidate_count).unwrap_or(0),
		top_k: u32::try_from(trace_row.top_k).unwrap_or(0),
		created_at: trace_row.created_at,
	};
	let created_at = context
		.created_at
		.format(&Rfc3339)
		.map_err(|err| eyre::eyre!("Failed to format trace created_at: {err}"))?;
	let candidates = decode_trace_replay_candidates(candidate_rows);
	let top_k = args.top_k.unwrap_or(context.top_k).max(1);
	let items_a = elf_service::search::replay_ranking_from_candidates(
		config_a,
		&context,
		None,
		&candidates,
		top_k,
	)
	.map_err(|err| eyre::eyre!("{err}"))?;
	let items_b = elf_service::search::replay_ranking_from_candidates(
		config_b,
		&context,
		None,
		&candidates,
		top_k,
	)
	.map_err(|err| eyre::eyre!("{err}"))?;
	let note_ids_a: Vec<Uuid> = items_a.iter().map(|item| item.note_id).collect();
	let note_ids_b: Vec<Uuid> = items_b.iter().map(|item| item.note_id).collect();
	let (positional_churn_at_k, set_churn_at_k) =
		churn_against_baseline_at_k(&note_ids_a, &note_ids_b, top_k as usize);
	let (retrieval_top3_total, a_retained, a_retention) =
		retrieval_top_rank_retention(&candidates, &note_ids_a, 3);
	let (_, b_retained, b_retention) = retrieval_top_rank_retention(&candidates, &note_ids_b, 3);
	let churn = TraceCompareChurn { positional_churn_at_k, set_churn_at_k };
	let guardrails = TraceCompareGuardrails {
		retrieval_top3_total,
		a_retrieval_top3_retained: a_retained,
		a_retrieval_top3_retention: a_retention,
		b_retrieval_top3_retained: b_retained,
		b_retrieval_top3_retention: b_retention,
		retrieval_top3_retention_delta: b_retention - a_retention,
	};
	let stage_deltas = build_trace_compare_stage_deltas(
		stage_rows.as_slice(),
		items_a.len() as u32,
		items_b.len() as u32,
	);
	let regression_attribution =
		build_trace_compare_regression_attribution(&churn, &guardrails, stage_deltas.as_slice());

	Ok(TraceCompareTrace {
		trace_id: context.trace_id,
		query: context.query,
		candidate_count: context.candidate_count,
		top_k,
		created_at,
		a: TraceCompareVariant { policy_id: policy_id_a.to_string(), items: items_a },
		b: TraceCompareVariant { policy_id: policy_id_b.to_string(), items: items_b },
		churn,
		guardrails,
		stage_deltas,
		regression_attribution,
	})
}

async fn fetch_trace_compare_trace_row(db: &Db, trace_id: &Uuid) -> Result<TraceCompareTraceRow> {
	let row: TraceCompareTraceRow = sqlx::query_as::<_, TraceCompareTraceRow>(
		"\
SELECT
	trace_id,
	query,
	candidate_count,
	top_k,
	created_at
FROM search_traces
WHERE trace_id = $1",
	)
	.bind(trace_id)
	.fetch_one(&db.pool)
	.await?;

	Ok(row)
}

async fn fetch_trace_compare_candidate_rows(
	db: &Db,
	trace_id: &Uuid,
) -> Result<Vec<TraceCompareCandidateRow>> {
	let rows: Vec<TraceCompareCandidateRow> = sqlx::query_as::<_, TraceCompareCandidateRow>(
		"\
SELECT
	candidate_snapshot,
	note_id,
	chunk_id,
	chunk_index,
	snippet,
	retrieval_rank,
	rerank_score,
	note_scope,
	note_importance,
	note_updated_at,
	note_hit_count,
	note_last_hit_at
FROM search_trace_candidates
WHERE trace_id = $1
ORDER BY retrieval_rank ASC",
	)
	.bind(trace_id)
	.fetch_all(&db.pool)
	.await?;

	Ok(rows)
}

async fn fetch_trace_compare_stage_rows(
	db: &Db,
	trace_id: &Uuid,
) -> Result<Vec<TraceCompareStageRow>> {
	let rows = sqlx::query_as::<_, TraceCompareStageRow>(
		"\
SELECT
	s.stage_order,
	s.stage_name,
	s.stage_payload,
	COUNT(i.id)::bigint AS item_count
FROM search_trace_stages s
LEFT JOIN search_trace_stage_items i ON i.stage_id = s.stage_id
WHERE s.trace_id = $1
GROUP BY s.stage_id, s.stage_order, s.stage_name, s.stage_payload
ORDER BY s.stage_order ASC",
	)
	.bind(trace_id)
	.fetch_all(&db.pool)
	.await?;

	Ok(rows)
}

async fn eval_config(
	config_path: &Path,
	config: Config,
	dataset: &EvalDataset,
	args: &Args,
) -> Result<EvalRun> {
	let db = Db::connect(&config.storage.postgres).await?;

	db.ensure_schema(config.storage.qdrant.vector_dim).await?;

	let qdrant = QdrantStore::new(&config.storage.qdrant)?;
	let service = ElfService::new(config, db, qdrant);
	let defaults = dataset.defaults.clone().unwrap_or(EvalDefaults {
		tenant_id: None,
		project_id: None,
		agent_id: None,
		read_profile: None,
		top_k: None,
		candidate_k: None,
		ranking: None,
	});
	let runs_per_query = args.runs_per_query.max(1);
	let mut reports = Vec::with_capacity(dataset.queries.len());
	let mut latencies_ms = Vec::with_capacity(dataset.queries.len());
	let mut stability_positional = Vec::new();
	let mut stability_set = Vec::new();

	for (index, query) in dataset.queries.iter().enumerate() {
		let merged = merge_query(&defaults, query, args, &service.cfg, index)?;
		let (first, latency_ms, stability, trace_ids) =
			run_query_n_times(&service, merged.request.clone(), runs_per_query).await?;
		let retrieved = unique_items(&first.items);
		let retrieved_note_ids: Vec<Uuid> = retrieved.iter().map(|item| item.note_id).collect();
		let retrieved_keys: Vec<Option<String>> =
			retrieved.iter().map(|item| item.key.clone()).collect();
		let retrieved_summary_chars =
			retrieved.iter().map(|item| item.summary.len()).sum::<usize>();
		let (metrics, expected_count) =
			compute_metrics_for_query(&merged, &retrieved_note_ids, &retrieved_keys);

		if let Some(s) = stability {
			stability_positional.push(s.positional_churn_at_k);
			stability_set.push(s.set_churn_at_k);
		}

		reports.push(QueryReport {
			id: merged.id,
			query: merged.query,
			trace_id: first.trace_id,
			trace_ids: (trace_ids.len() > 1).then_some(trace_ids),
			expected_count,
			retrieved_count: retrieved_note_ids.len(),
			relevant_count: metrics.relevant_count,
			recall_at_k: metrics.recall_at_k,
			precision_at_k: metrics.precision_at_k,
			rr: metrics.rr,
			ndcg: metrics.ndcg,
			latency_ms,
			expected_note_ids: merged.expected_note_ids,
			expected_keys: merged.expected_keys,
			expected_kind: merged.expected_kind,
			retrieved_note_ids,
			retrieved_keys: if merged.expected_kind == ExpectedKind::Key {
				retrieved_keys
			} else {
				Vec::new()
			},
			retrieved_summary_chars,
			stability,
		});
		latencies_ms.push(latency_ms);
	}

	let mut summary = summarize(&reports, &latencies_ms);

	if runs_per_query > 1 && !stability_positional.is_empty() {
		let count = stability_positional.len().max(1) as f64;
		let avg_positional_churn_at_k = stability_positional.iter().sum::<f64>() / count;
		let avg_set_churn_at_k = stability_set.iter().sum::<f64>() / count;

		summary.stability = Some(StabilitySummary {
			runs_per_query,
			avg_positional_churn_at_k,
			avg_set_churn_at_k,
		});
	}

	let settings = EvalSettings {
		config_path: config_path.display().to_string(),
		candidate_k: args
			.candidate_k
			.or(dataset.defaults.as_ref().and_then(|d| d.candidate_k))
			.unwrap_or(service.cfg.memory.candidate_k),
		top_k: args
			.top_k
			.or(dataset.defaults.as_ref().and_then(|d| d.top_k))
			.unwrap_or(service.cfg.memory.top_k),
		runs_per_query: (runs_per_query > 1).then_some(runs_per_query),
	};

	Ok(EvalRun {
		dataset: EvalDatasetInfo {
			name: dataset.name.clone().unwrap_or_else(|| "eval".to_string()),
			query_count: reports.len(),
		},
		settings,
		summary,
		queries: reports,
	})
}

async fn run_query_n_times(
	service: &ElfService,
	request: SearchRequest,
	runs_per_query: u32,
) -> Result<(SearchIndexResponse, f64, Option<QueryStability>, Vec<Uuid>)> {
	let k = request.top_k.unwrap_or(1).max(1) as usize;
	let runs = runs_per_query.max(1);
	let mut first_response: Option<SearchIndexResponse> = None;
	let mut first_retrieved_ids: Vec<Uuid> = Vec::new();
	let mut trace_ids: Vec<Uuid> = Vec::with_capacity(runs as usize);
	let mut latency_total_ms = 0.0_f64;
	let mut positional_churn_sum = 0.0_f64;
	let mut set_churn_sum = 0.0_f64;
	let mut churn_count = 0_u32;

	for run_idx in 0..runs {
		let start = Instant::now();
		let response = service.search(request.clone()).await?;
		let latency_ms = start.elapsed().as_secs_f64() * 1_000.0;

		latency_total_ms += latency_ms;

		trace_ids.push(response.trace_id);

		let retrieved = unique_items(&response.items);
		let retrieved_ids = retrieved.iter().map(|item| item.note_id).collect::<Vec<_>>();

		if run_idx == 0 {
			first_retrieved_ids = retrieved_ids;
			first_response = Some(response);

			continue;
		}

		let (positional_churn_at_k, set_churn_at_k) =
			churn_against_baseline_at_k(&first_retrieved_ids, &retrieved_ids, k);

		positional_churn_sum += positional_churn_at_k;
		set_churn_sum += set_churn_at_k;
		churn_count += 1;
	}

	let latency_ms_mean = latency_total_ms / runs as f64;
	let stability = if churn_count > 0 {
		Some(QueryStability {
			runs_per_query: runs,
			positional_churn_at_k: positional_churn_sum / churn_count as f64,
			set_churn_at_k: set_churn_sum / churn_count as f64,
		})
	} else {
		None
	};

	Ok((
		first_response.ok_or_else(|| eyre::eyre!("No search responses were collected."))?,
		latency_ms_mean,
		stability,
		trace_ids,
	))
}

#[cfg(test)]
mod tests {
	use std::collections::HashSet;

	use crate::{
		ExpectedKind, OffsetDateTime, Uuid, compute_metrics_for_keys, resolve_expected_mode,
		retrieval_top_rank_retention,
	};

	#[test]
	fn resolve_expected_mode_requires_exactly_one_definition() {
		let index = 0;
		let note_ids = vec![Uuid::new_v4()];
		let expected_keys = vec!["key-1".to_string()];
		let note_only = resolve_expected_mode(index, &note_ids, &[]);
		let key_only = resolve_expected_mode(index, &[], &expected_keys);
		let none = resolve_expected_mode(index, &[], &[]);
		let both = resolve_expected_mode(index, &note_ids, &expected_keys);

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
		let metrics = compute_metrics_for_keys(&retrieved, &expected);
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
		let (total, retained, retention) = retrieval_top_rank_retention(&candidates, &note_ids, 3);

		assert_eq!(total, 2);
		assert_eq!(retained, 1);
		assert!((retention - 0.5).abs() < 1e-12, "Unexpected retention: {retention}");
	}
}
