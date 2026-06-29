use color_eyre::{Result, eyre};
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use elf_config::Config;
use elf_service::search::{self, TraceReplayContext};
use elf_storage::db::Db;

use super::{
	cli::Args,
	gate::{GateThresholds, GateTrace},
	replay::{
		churn_against_baseline_at_k, decode_trace_replay_candidates, retrieval_top_rank_retention,
	},
	reports::{GateBreach, TraceChurn, TraceReport, TraceRetention},
	storage::{fetch_baseline_items, fetch_candidate_rows, fetch_trace_row},
};

pub(super) async fn eval_trace(
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
	let context = TraceReplayContext {
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
	let replay_items =
		search::replay_ranking_from_candidates(cfg, &context, None, &candidates, top_k)
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
