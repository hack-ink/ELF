use std::{
	collections::HashSet,
	fs,
	path::{Path, PathBuf},
	time::Instant,
};

use clap::Parser;
use color_eyre::eyre;
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use elf_config::Config;
use elf_service::{ElfService, RankingRequestOverride, SearchIndexResponse, SearchRequest};
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
	expected_note_ids: Vec<Uuid>,
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
	retrieved_note_ids: Vec<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	stability: Option<QueryStability>,
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
}

#[derive(Debug, Serialize)]
struct TraceCompareVariant {
	policy_id: String,
	items: Vec<elf_service::search::TraceReplayItem>,
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

#[derive(sqlx::FromRow)]
struct TraceCompareTraceRow {
	trace_id: Uuid,
	query: String,
	candidate_count: i32,
	top_k: i32,
	created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct TraceCompareCandidateRow {
	note_id: Uuid,
	chunk_id: Uuid,
	retrieval_rank: i32,
	rerank_score: f32,
	note_scope: String,
	note_importance: f32,
	note_updated_at: OffsetDateTime,
}

struct MergedQuery {
	id: String,
	query: String,
	expected_note_ids: Vec<Uuid>,
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

pub async fn run(args: Args) -> color_eyre::Result<()> {
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

async fn trace_compare(
	config_a_path: &Path,
	config_a: Config,
	config_b_path: &Path,
	config_b: Config,
	args: &Args,
) -> color_eyre::Result<TraceCompareOutput> {
	let policy_id_a = elf_service::search::ranking_policy_id(&config_a, None)
		.map_err(|err| eyre::eyre!("{err}"))?;
	let policy_id_b = elf_service::search::ranking_policy_id(&config_b, None)
		.map_err(|err| eyre::eyre!("{err}"))?;
	let db = Db::connect(&config_a.storage.postgres).await?;
	let mut traces = Vec::with_capacity(args.trace_id.len());
	let mut positional_sum = 0.0_f64;
	let mut set_sum = 0.0_f64;
	let mut top3_retention_a_sum = 0.0_f64;
	let mut top3_retention_b_sum = 0.0_f64;

	for trace_id in &args.trace_id {
		let trace_row: TraceCompareTraceRow = sqlx::query_as(
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

		let candidate_rows: Vec<TraceCompareCandidateRow> = sqlx::query_as(
			"\
SELECT
	note_id,
	chunk_id,
	retrieval_rank,
	rerank_score,
	note_scope,
	note_importance,
	note_updated_at
FROM search_trace_candidates
WHERE trace_id = $1
ORDER BY retrieval_rank ASC",
		)
		.bind(trace_id)
		.fetch_all(&db.pool)
		.await?;
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
		let candidates: Vec<elf_service::search::TraceReplayCandidate> = candidate_rows
			.into_iter()
			.map(|row| elf_service::search::TraceReplayCandidate {
				note_id: row.note_id,
				chunk_id: row.chunk_id,
				retrieval_rank: u32::try_from(row.retrieval_rank).unwrap_or(0),
				rerank_score: row.rerank_score,
				note_scope: row.note_scope,
				note_importance: row.note_importance,
				note_updated_at: row.note_updated_at,
			})
			.collect();
		let top_k = args.top_k.unwrap_or(context.top_k).max(1);
		let items_a = elf_service::search::replay_ranking_from_candidates(
			&config_a,
			&context,
			None,
			&candidates,
			top_k,
		)
		.map_err(|err| eyre::eyre!("{err}"))?;
		let items_b = elf_service::search::replay_ranking_from_candidates(
			&config_b,
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
		let (_, b_retained, b_retention) =
			retrieval_top_rank_retention(&candidates, &note_ids_b, 3);
		let retention_delta = b_retention - a_retention;

		positional_sum += positional_churn_at_k;
		set_sum += set_churn_at_k;
		top3_retention_a_sum += a_retention;
		top3_retention_b_sum += b_retention;

		traces.push(TraceCompareTrace {
			trace_id: context.trace_id,
			query: context.query,
			candidate_count: context.candidate_count,
			top_k,
			created_at,
			a: TraceCompareVariant { policy_id: policy_id_a.clone(), items: items_a },
			b: TraceCompareVariant { policy_id: policy_id_b.clone(), items: items_b },
			churn: TraceCompareChurn { positional_churn_at_k, set_churn_at_k },
			guardrails: TraceCompareGuardrails {
				retrieval_top3_total,
				a_retrieval_top3_retained: a_retained,
				a_retrieval_top3_retention: a_retention,
				b_retrieval_top3_retained: b_retained,
				b_retrieval_top3_retention: b_retention,
				retrieval_top3_retention_delta: retention_delta,
			},
		});
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

fn load_dataset(path: &Path) -> color_eyre::Result<EvalDataset> {
	let raw = fs::read_to_string(path)?;
	let dataset: EvalDataset = serde_json::from_str(&raw)?;

	if dataset.queries.is_empty() {
		return Err(eyre::eyre!("Dataset must include at least one query."));
	}

	Ok(dataset)
}

async fn eval_config(
	config_path: &Path,
	config: Config,
	dataset: &EvalDataset,
	args: &Args,
) -> color_eyre::Result<EvalRun> {
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

	let mut reports = Vec::with_capacity(dataset.queries.len());
	let mut latencies_ms = Vec::with_capacity(dataset.queries.len());
	let mut stability_positional = Vec::new();
	let mut stability_set = Vec::new();

	let runs_per_query = args.runs_per_query.max(1);

	for (index, query) in dataset.queries.iter().enumerate() {
		let merged = merge_query(&defaults, query, args, &service.cfg, index)?;
		let expected: HashSet<Uuid> = merged.expected_note_ids.iter().copied().collect();
		let (first, latency_ms, stability, trace_ids) =
			run_query_n_times(&service, merged.request, runs_per_query).await?;
		let retrieved = unique_ids(first.items.iter().map(|item| item.note_id));
		let metrics = compute_metrics(&retrieved, &expected);

		if let Some(s) = stability {
			stability_positional.push(s.positional_churn_at_k);
			stability_set.push(s.set_churn_at_k);
		}

		reports.push(QueryReport {
			id: merged.id,
			query: merged.query,
			trace_id: first.trace_id,
			trace_ids: (trace_ids.len() > 1).then_some(trace_ids),
			expected_count: expected.len(),
			retrieved_count: retrieved.len(),
			relevant_count: metrics.relevant_count,
			recall_at_k: metrics.recall_at_k,
			precision_at_k: metrics.precision_at_k,
			rr: metrics.rr,
			ndcg: metrics.ndcg,
			latency_ms,
			expected_note_ids: merged.expected_note_ids,
			retrieved_note_ids: retrieved,
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
) -> color_eyre::Result<(SearchIndexResponse, f64, Option<QueryStability>, Vec<Uuid>)> {
	let k = request.top_k.unwrap_or(1).max(1) as usize;
	let runs = runs_per_query.max(1);

	let mut first_response: Option<SearchIndexResponse> = None;
	let mut first_retrieved: Vec<Uuid> = Vec::new();
	let mut trace_ids: Vec<Uuid> = Vec::with_capacity(runs as usize);
	let mut latency_total_ms = 0.0_f64;
	let mut positional_churn_sum = 0.0_f64;
	let mut set_churn_sum = 0.0_f64;
	let mut churn_count = 0u32;

	for run_idx in 0..runs {
		let start = Instant::now();
		let response = service.search(request.clone()).await?;
		let latency_ms = start.elapsed().as_secs_f64() * 1_000.0;

		latency_total_ms += latency_ms;
		trace_ids.push(response.trace_id);

		let retrieved = unique_ids(response.items.iter().map(|item| item.note_id));

		if run_idx == 0 {
			first_retrieved = retrieved;
			first_response = Some(response);
			continue;
		}

		let (positional_churn_at_k, set_churn_at_k) =
			churn_against_baseline_at_k(&first_retrieved, &retrieved, k);

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

fn churn_against_baseline_at_k(baseline: &[Uuid], other: &[Uuid], k: usize) -> (f64, f64) {
	let k = k.max(1);

	let mut positional_diff = 0usize;

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
) -> color_eyre::Result<MergedQuery> {
	if query.expected_note_ids.is_empty() {
		return Err(eyre::eyre!(
			"Query at index {index} must include at least one expected_note_id."
		));
	}

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
		request: SearchRequest {
			tenant_id,
			project_id,
			agent_id,
			read_profile,
			query: query.query.clone(),
			top_k: Some(top_k),
			candidate_k: Some(candidate_k),
			record_hits: Some(false),
			ranking,
		},
	})
}

fn unique_ids<I>(iter: I) -> Vec<Uuid>
where
	I: Iterator<Item = Uuid>,
{
	let mut seen = HashSet::new();
	let mut out = Vec::new();

	for id in iter {
		if seen.insert(id) {
			out.push(id);
		}
	}

	out
}

fn compute_metrics(retrieved: &[Uuid], expected: &HashSet<Uuid>) -> Metrics {
	let expected_count = expected.len();

	let mut relevant_count = 0usize;
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

fn summarize(reports: &[QueryReport], latencies_ms: &[f64]) -> EvalSummary {
	let count = reports.len().max(1) as f64;
	let avg_recall_at_k = reports.iter().map(|r| r.recall_at_k).sum::<f64>() / count;
	let avg_precision_at_k = reports.iter().map(|r| r.precision_at_k).sum::<f64>() / count;
	let mean_rr = reports.iter().map(|r| r.rr).sum::<f64>() / count;
	let mean_ndcg = reports.iter().map(|r| r.ndcg).sum::<f64>() / count;

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

#[cfg(test)]
mod tests {
	use super::*;

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
				retrieval_rank: 1,
				rerank_score: 0.1,
				note_scope: "project_shared".to_string(),
				note_importance: 0.1,
				note_updated_at: now,
			},
			elf_service::search::TraceReplayCandidate {
				note_id: note_a,
				chunk_id: Uuid::new_v4(),
				retrieval_rank: 2,
				rerank_score: 0.2,
				note_scope: "project_shared".to_string(),
				note_importance: 0.1,
				note_updated_at: now,
			},
			elf_service::search::TraceReplayCandidate {
				note_id: note_b,
				chunk_id: Uuid::new_v4(),
				retrieval_rank: 3,
				rerank_score: 0.3,
				note_scope: "org_shared".to_string(),
				note_importance: 0.1,
				note_updated_at: now,
			},
			elf_service::search::TraceReplayCandidate {
				note_id: note_c,
				chunk_id: Uuid::new_v4(),
				retrieval_rank: 4,
				rerank_score: 0.4,
				note_scope: "org_shared".to_string(),
				note_importance: 0.1,
				note_updated_at: now,
			},
		];
		let note_ids = vec![note_a, note_c];

		let (total, retained, retention) = retrieval_top_rank_retention(&candidates, &note_ids, 3);

		assert_eq!(total, 2);
		assert_eq!(retained, 1);
		assert!((retention - 0.5).abs() < 1e-12, "Unexpected retention: {retention}");
	}
}
