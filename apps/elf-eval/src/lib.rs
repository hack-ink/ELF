use std::{collections::HashSet, fs, path::PathBuf, time::Instant};

use clap::Parser;
use color_eyre::eyre;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use elf_service::ElfService;
use elf_storage::{db::Db, qdrant::QdrantStore};

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
pub struct Args {
	#[arg(long, short = 'c', value_name = "FILE")]
	pub config: PathBuf,
	#[arg(long, short = 'd', value_name = "FILE")]
	pub dataset: PathBuf,
	#[arg(long, value_name = "N")]
	pub top_k: Option<u32>,
	#[arg(long, value_name = "N")]
	pub candidate_k: Option<u32>,
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
}

#[derive(Debug, Serialize)]
struct EvalSummary {
	avg_recall_at_k: f64,
	avg_precision_at_k: f64,
	mean_rr: f64,
	mean_ndcg: f64,
	latency_ms_p50: f64,
	latency_ms_p95: f64,
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
}

struct MergedQuery {
	id: String,
	query: String,
	expected_note_ids: Vec<Uuid>,
	request: elf_service::SearchRequest,
}

struct Metrics {
	recall_at_k: f64,
	precision_at_k: f64,
	rr: f64,
	ndcg: f64,
	relevant_count: usize,
}

pub async fn run(args: Args) -> color_eyre::Result<()> {
	let config = elf_config::load(&args.config)?;
	let filter = EnvFilter::new(config.service.log_level.clone());

	tracing_subscriber::fmt().with_env_filter(filter).init();

	let db = Db::connect(&config.storage.postgres).await?;

	db.ensure_schema(config.storage.qdrant.vector_dim).await?;

	let qdrant = QdrantStore::new(&config.storage.qdrant)?;
	let service = ElfService::new(config, db, qdrant);
	let dataset = load_dataset(&args.dataset)?;
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

	for (index, query) in dataset.queries.iter().enumerate() {
		let merged = merge_query(&defaults, query, &args, &service.cfg, index)?;
		let start = Instant::now();
		let response = service.search(merged.request).await?;
		let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
		let retrieved = unique_ids(response.items.iter().map(|item| item.note_id));
		let expected: HashSet<Uuid> = merged.expected_note_ids.iter().copied().collect();
		let metrics = compute_metrics(&retrieved, &expected);

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
		});

		latencies_ms.push(latency_ms);
	}

	let summary = summarize(&reports, &latencies_ms);
	let output = EvalOutput {
		dataset: EvalDatasetInfo {
			name: dataset.name.unwrap_or_else(|| "eval".to_string()),
			query_count: reports.len(),
		},
		settings: EvalSettings {
			config_path: args.config.display().to_string(),
			candidate_k: args
				.candidate_k
				.or(dataset.defaults.as_ref().and_then(|d| d.candidate_k))
				.unwrap_or(service.cfg.memory.candidate_k),
			top_k: args
				.top_k
				.or(dataset.defaults.as_ref().and_then(|d| d.top_k))
				.unwrap_or(service.cfg.memory.top_k),
		},
		summary,
		queries: reports,
	};
	let json = serde_json::to_string_pretty(&output)?;

	println!("{json}");

	Ok(())
}

fn load_dataset(path: &PathBuf) -> color_eyre::Result<EvalDataset> {
	let raw = fs::read_to_string(path)?;
	let dataset: EvalDataset = serde_json::from_str(&raw)?;

	if dataset.queries.is_empty() {
		return Err(eyre::eyre!("Dataset must include at least one query."));
	}

	Ok(dataset)
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
		request: elf_service::SearchRequest {
			tenant_id,
			project_id,
			agent_id,
			read_profile,
			query: query.query.clone(),
			top_k: Some(top_k),
			candidate_k: Some(candidate_k),
			record_hits: Some(false),
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
