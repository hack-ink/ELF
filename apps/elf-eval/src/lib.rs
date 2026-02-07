use std::{
	collections::HashSet,
	fs,
	path::{Path, PathBuf},
	time::Instant,
};

use clap::Parser;
use color_eyre::eyre;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use elf_service::{ElfService, SearchIndexResponse, SearchRequest};
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
	#[arg(long, short = 'd', value_name = "FILE")]
	pub dataset: PathBuf,
	#[arg(long, value_name = "N")]
	pub top_k: Option<u32>,
	#[arg(long, value_name = "N")]
	pub candidate_k: Option<u32>,
	#[arg(long, value_name = "N", default_value_t = 1)]
	pub runs_per_query: u32,
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
	queries: Vec<CompareQueryReport>,
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
}

#[derive(Debug, Serialize)]
struct QueryVariantReport {
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

	let dataset = load_dataset(args.dataset.as_path())?;
	let run_a = eval_config(args.config_a.as_path(), config_a, &dataset, &args).await?;

	if let Some(config_b_path) = &args.config_b {
		let config_b = elf_config::load(config_b_path)?;
		let run_b = eval_config(config_b_path.as_path(), config_b, &dataset, &args).await?;
		let queries = build_compare_queries(&run_a.queries, &run_b.queries);
		let summary_delta = diff_summary(&run_a.summary, &run_b.summary);
		let output = CompareOutput {
			dataset: run_a.dataset,
			settings_a: run_a.settings,
			settings_b: run_b.settings,
			summary_a: run_a.summary,
			summary_b: run_b.summary,
			summary_delta,
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
	config: elf_config::Config,
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
	});

	let mut reports = Vec::with_capacity(dataset.queries.len());
	let mut latencies_ms = Vec::with_capacity(dataset.queries.len());
	let mut stability_positional = Vec::new();
	let mut stability_set = Vec::new();

	let runs_per_query = args.runs_per_query.max(1);

	for (index, query) in dataset.queries.iter().enumerate() {
		let merged = merge_query(&defaults, query, args, &service.cfg, index)?;
		let expected: HashSet<Uuid> = merged.expected_note_ids.iter().copied().collect();
		let (first, latency_ms, stability) =
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
) -> color_eyre::Result<(SearchIndexResponse, f64, Option<QueryStability>)> {
	let k = request.top_k.unwrap_or(1).max(1) as usize;
	let runs = runs_per_query.max(1);

	let mut first_response: Option<SearchIndexResponse> = None;
	let mut first_retrieved: Vec<Uuid> = Vec::new();
	let mut latency_total_ms = 0.0_f64;
	let mut positional_churn_sum = 0.0_f64;
	let mut set_churn_sum = 0.0_f64;
	let mut churn_count = 0u32;

	for run_idx in 0..runs {
		let start = Instant::now();
		let response = service.search(request.clone()).await?;
		let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

		latency_total_ms += latency_ms;

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

fn build_compare_queries(a: &[QueryReport], b: &[QueryReport]) -> Vec<CompareQueryReport> {
	a.iter()
		.zip(b.iter())
		.map(|(qa, qb)| {
			let delta_stability = match (qa.stability, qb.stability) {
				(Some(sa), Some(sb)) => Some(QueryStabilityDelta {
					positional_churn_at_k: sb.positional_churn_at_k - sa.positional_churn_at_k,
					set_churn_at_k: sb.set_churn_at_k - sa.set_churn_at_k,
				}),
				_ => None,
			};

			CompareQueryReport {
				id: qa.id.clone(),
				query: qa.query.clone(),
				expected_count: qa.expected_count,
				expected_note_ids: qa.expected_note_ids.clone(),
				a: QueryVariantReport {
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
			}
		})
		.collect()
}

fn merge_query(
	defaults: &EvalDefaults,
	query: &EvalQuery,
	args: &Args,
	cfg: &elf_config::Config,
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
			ranking: None,
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
