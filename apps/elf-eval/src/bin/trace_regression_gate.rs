use std::{collections::HashSet, fs, path::PathBuf};

use clap::Parser;
use color_eyre::{Result, eyre};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use elf_config::Config;
use elf_storage::db::Db;

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
struct Args {
	#[arg(long, short = 'c', value_name = "FILE")]
	config: PathBuf,
	#[arg(long, short = 'g', value_name = "FILE")]
	gate: PathBuf,
	#[arg(long, value_name = "FILE")]
	out: Option<PathBuf>,
	#[arg(long, value_name = "N")]
	top_k: Option<u32>,
	#[arg(long, value_name = "N")]
	retrieval_retention_rank: Option<u32>,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "snake_case")]
struct GateThresholds {
	max_positional_churn_at_k: Option<f64>,
	max_set_churn_at_k: Option<f64>,
	min_retrieval_top_rank_retention: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
struct GateTrace {
	trace_id: Uuid,
	top_k: Option<u32>,
	retrieval_retention_rank: Option<u32>,
	#[serde(flatten)]
	thresholds: GateThresholds,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct GateFile {
	#[serde(default)]
	defaults: GateThresholds,
	top_k: Option<u32>,
	retrieval_retention_rank: Option<u32>,
	traces: Vec<GateTrace>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct GateReport {
	config_path: String,
	gate_path: String,
	summary: GateSummary,
	traces: Vec<TraceReport>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct GateSummary {
	trace_count: usize,
	breached_count: usize,
	ok: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct TraceReport {
	trace_id: Uuid,
	query: String,
	created_at: String,
	top_k: u32,
	retrieval_retention_rank: u32,
	candidate_count: u32,
	baseline_count: usize,
	replay_count: usize,
	churn: TraceChurn,
	retention: TraceRetention,
	breaches: Vec<GateBreach>,
	ok: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct TraceChurn {
	positional_churn_at_k: f64,
	set_churn_at_k: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct TraceRetention {
	retrieval_top_rank_total: usize,
	baseline_retrieval_top_rank_retained: usize,
	baseline_retrieval_top_rank_retention: f64,
	replay_retrieval_top_rank_retained: usize,
	replay_retrieval_top_rank_retention: f64,
	retention_delta: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct GateBreach {
	metric: String,
	value: f64,
	threshold: f64,
	op: String,
}

#[derive(Debug, FromRow)]
struct TraceRow {
	trace_id: Uuid,
	query: String,
	candidate_count: i32,
	top_k: i32,
	created_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
struct TraceItemRow {
	note_id: Uuid,
}

#[derive(Debug, FromRow)]
struct CandidateRow {
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

fn load_gate_file(path: &PathBuf) -> Result<GateFile> {
	let raw = fs::read_to_string(path)?;
	let out: GateFile = serde_json::from_str(&raw)?;

	Ok(out)
}

fn merge_thresholds(defaults: GateThresholds, overrides: GateThresholds) -> GateThresholds {
	GateThresholds {
		max_positional_churn_at_k: overrides
			.max_positional_churn_at_k
			.or(defaults.max_positional_churn_at_k),
		max_set_churn_at_k: overrides.max_set_churn_at_k.or(defaults.max_set_churn_at_k),
		min_retrieval_top_rank_retention: overrides
			.min_retrieval_top_rank_retention
			.or(defaults.min_retrieval_top_rank_retention),
	}
}

fn decode_trace_replay_candidates(
	rows: Vec<CandidateRow>,
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

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	let args = Args::parse();
	let cfg = elf_config::load(&args.config)?;
	let filter = EnvFilter::new(cfg.service.log_level.clone());

	tracing_subscriber::fmt().with_env_filter(filter).init();

	let gate = load_gate_file(&args.gate)?;

	if gate.traces.is_empty() {
		return Err(eyre::eyre!("Gate JSON must include at least one trace."));
	}

	let gate_top_k = gate.top_k;
	let gate_retrieval_retention_rank = gate.retrieval_retention_rank;
	let db = Db::connect(&cfg.storage.postgres).await?;

	db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;

	let mut traces = Vec::with_capacity(gate.traces.len());
	let mut breached_count = 0_usize;

	for trace in gate.traces {
		let thresholds = merge_thresholds(gate.defaults, trace.thresholds);
		let report = eval_trace(
			&db,
			&cfg,
			&args,
			gate_top_k,
			gate_retrieval_retention_rank,
			&trace,
			thresholds,
		)
		.await?;

		if !report.ok {
			breached_count += 1;
		}

		traces.push(report);
	}

	let summary =
		GateSummary { trace_count: traces.len(), breached_count, ok: breached_count == 0 };
	let report = GateReport {
		config_path: args.config.display().to_string(),
		gate_path: args.gate.display().to_string(),
		summary,
		traces,
	};
	let json = serde_json::to_string_pretty(&report)?;

	if let Some(out_path) = &args.out {
		fs::write(out_path, &json)?;
	} else {
		println!("{json}");
	}

	if !report.summary.ok {
		return Err(eyre::eyre!(
			"Trace regression gate breached: {}/{} traces failed thresholds.",
			report.summary.breached_count,
			report.summary.trace_count
		));
	}

	Ok(())
}

async fn eval_trace(
	db: &Db,
	cfg: &Config,
	cli: &Args,
	gate_top_k: Option<u32>,
	gate_retrieval_retention_rank: Option<u32>,
	trace: &GateTrace,
	thresholds: GateThresholds,
) -> Result<TraceReport> {
	let trace_row = fetch_trace_row(db, &trace.trace_id).await?;
	let created_at = trace_row
		.created_at
		.format(&Rfc3339)
		.map_err(|err| eyre::eyre!("Failed to format created_at: {err}"))?;
	let context = elf_service::search::TraceReplayContext {
		trace_id: trace_row.trace_id,
		query: trace_row.query.clone(),
		candidate_count: u32::try_from(trace_row.candidate_count).unwrap_or(0),
		top_k: u32::try_from(trace_row.top_k).unwrap_or(0),
		created_at: trace_row.created_at,
	};
	let top_k =
		trace.top_k.or(cli.top_k).or(gate_top_k).or(Some(context.top_k)).unwrap_or(10).max(1);
	let retrieval_retention_rank = trace
		.retrieval_retention_rank
		.or(cli.retrieval_retention_rank)
		.or(gate_retrieval_retention_rank)
		.unwrap_or(3)
		.max(1);
	let baseline_items = fetch_baseline_items(db, &trace.trace_id, top_k).await?;
	let baseline_note_ids: Vec<Uuid> = baseline_items.iter().map(|row| row.note_id).collect();
	let candidate_rows = fetch_candidate_rows(db, &trace.trace_id).await?;
	let candidates = decode_trace_replay_candidates(candidate_rows);
	let replay_items = elf_service::search::replay_ranking_from_candidates(
		cfg,
		&context,
		None,
		&candidates,
		top_k,
	)
	.map_err(|err| eyre::eyre!("{err}"))?;
	let replay_note_ids: Vec<Uuid> = replay_items.iter().map(|item| item.note_id).collect();
	let effective_k = top_k as usize;
	let (positional_churn_at_k, set_churn_at_k) =
		churn_against_baseline_at_k(&baseline_note_ids, &replay_note_ids, effective_k);
	let churn = TraceChurn { positional_churn_at_k, set_churn_at_k };
	let (retrieval_top_rank_total, baseline_retained, baseline_retention) =
		retrieval_top_rank_retention(&candidates, &baseline_note_ids, retrieval_retention_rank);
	let (_, replay_retained, replay_retention) =
		retrieval_top_rank_retention(&candidates, &replay_note_ids, retrieval_retention_rank);
	let retention = TraceRetention {
		retrieval_top_rank_total,
		baseline_retrieval_top_rank_retained: baseline_retained,
		baseline_retrieval_top_rank_retention: baseline_retention,
		replay_retrieval_top_rank_retained: replay_retained,
		replay_retrieval_top_rank_retention: replay_retention,
		retention_delta: replay_retention - baseline_retention,
	};
	let mut breaches = Vec::new();

	if baseline_note_ids.len() < effective_k {
		breaches.push(GateBreach {
			metric: "baseline_count_at_k".to_string(),
			value: baseline_note_ids.len() as f64,
			threshold: effective_k as f64,
			op: ">=".to_string(),
		});
	}
	if replay_note_ids.len() < effective_k {
		breaches.push(GateBreach {
			metric: "replay_count_at_k".to_string(),
			value: replay_note_ids.len() as f64,
			threshold: effective_k as f64,
			op: ">=".to_string(),
		});
	}

	if let Some(max) = thresholds.max_positional_churn_at_k
		&& churn.positional_churn_at_k > max
	{
		breaches.push(GateBreach {
			metric: "positional_churn_at_k".to_string(),
			value: churn.positional_churn_at_k,
			threshold: max,
			op: "<=".to_string(),
		});
	}
	if let Some(max) = thresholds.max_set_churn_at_k
		&& churn.set_churn_at_k > max
	{
		breaches.push(GateBreach {
			metric: "set_churn_at_k".to_string(),
			value: churn.set_churn_at_k,
			threshold: max,
			op: "<=".to_string(),
		});
	}
	if let Some(min) = thresholds.min_retrieval_top_rank_retention
		&& retention.replay_retrieval_top_rank_retention < min
	{
		breaches.push(GateBreach {
			metric: "replay_retrieval_top_rank_retention".to_string(),
			value: retention.replay_retrieval_top_rank_retention,
			threshold: min,
			op: ">=".to_string(),
		});
	}

	Ok(TraceReport {
		trace_id: trace.trace_id,
		query: context.query,
		created_at,
		top_k,
		retrieval_retention_rank,
		candidate_count: context.candidate_count,
		baseline_count: baseline_note_ids.len(),
		replay_count: replay_note_ids.len(),
		churn,
		retention,
		ok: breaches.is_empty(),
		breaches,
	})
}

async fn fetch_trace_row(db: &Db, trace_id: &Uuid) -> Result<TraceRow> {
	let row: TraceRow = sqlx::query_as::<_, TraceRow>(
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

async fn fetch_baseline_items(db: &Db, trace_id: &Uuid, top_k: u32) -> Result<Vec<TraceItemRow>> {
	let rows: Vec<TraceItemRow> = sqlx::query_as::<_, TraceItemRow>(
		"\
SELECT
	note_id
FROM search_trace_items
WHERE trace_id = $1
ORDER BY rank ASC
LIMIT $2",
	)
	.bind(trace_id)
	.bind(i64::from(top_k.max(1)))
	.fetch_all(&db.pool)
	.await?;

	Ok(rows)
}

async fn fetch_candidate_rows(db: &Db, trace_id: &Uuid) -> Result<Vec<CandidateRow>> {
	let rows: Vec<CandidateRow> = sqlx::query_as::<_, CandidateRow>(
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
