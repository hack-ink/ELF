mod analysis;

use std::path::Path;

use color_eyre::{Result, eyre};
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::app::{
	Args,
	metrics::{self},
	types::{
		TraceCompareCandidateRow, TraceCompareChurn, TraceCompareGuardrails, TraceCompareOutput,
		TraceComparePolicies, TraceComparePolicy, TraceCompareStageRow, TraceCompareSummary,
		TraceCompareTrace, TraceCompareTraceRow, TraceCompareVariant,
	},
};
use elf_config::Config;
use elf_service::search::{self, TraceReplayContext};
use elf_storage::db::Db;

pub(super) async fn trace_compare(
	config_a_path: &Path,
	config_a: Config,
	config_b_path: &Path,
	config_b: Config,
	args: &Args,
) -> Result<TraceCompareOutput> {
	let policy_id_a =
		search::ranking_policy_id(&config_a, None).map_err(|err| eyre::eyre!("{err}"))?;
	let policy_id_b =
		search::ranking_policy_id(&config_b, None).map_err(|err| eyre::eyre!("{err}"))?;
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
	let context = TraceReplayContext {
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
	let candidates = analysis::decode_trace_replay_candidates(candidate_rows);
	let top_k = args.top_k.unwrap_or(context.top_k).max(1);
	let items_a =
		search::replay_ranking_from_candidates(config_a, &context, None, &candidates, top_k)
			.map_err(|err| eyre::eyre!("{err}"))?;
	let items_b =
		search::replay_ranking_from_candidates(config_b, &context, None, &candidates, top_k)
			.map_err(|err| eyre::eyre!("{err}"))?;
	let note_ids_a: Vec<Uuid> = items_a.iter().map(|item| item.note_id).collect();
	let note_ids_b: Vec<Uuid> = items_b.iter().map(|item| item.note_id).collect();
	let (positional_churn_at_k, set_churn_at_k) =
		metrics::churn_against_baseline_at_k(&note_ids_a, &note_ids_b, top_k as usize);
	let (retrieval_top3_total, a_retained, a_retention) =
		metrics::retrieval_top_rank_retention(&candidates, &note_ids_a, 3);
	let (_, b_retained, b_retention) =
		metrics::retrieval_top_rank_retention(&candidates, &note_ids_b, 3);
	let churn = TraceCompareChurn { positional_churn_at_k, set_churn_at_k };
	let guardrails = TraceCompareGuardrails {
		retrieval_top3_total,
		a_retrieval_top3_retained: a_retained,
		a_retrieval_top3_retention: a_retention,
		b_retrieval_top3_retained: b_retained,
		b_retrieval_top3_retention: b_retention,
		retrieval_top3_retention_delta: b_retention - a_retention,
	};
	let stage_deltas = analysis::build_trace_compare_stage_deltas(
		stage_rows.as_slice(),
		items_a.len() as u32,
		items_b.len() as u32,
	);
	let regression_attribution = analysis::build_trace_compare_regression_attribution(
		&churn,
		&guardrails,
		stage_deltas.as_slice(),
	);

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
