use std::{collections::HashMap, path::Path};

use color_eyre::{Result, eyre};
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use elf_config::Config;
use elf_service::search::{self, TraceReplayCandidate, TraceReplayContext};
use elf_storage::db::Db;

use super::{
	Args,
	metrics::{churn_against_baseline_at_k, retrieval_top_rank_retention},
	types::{
		TraceCompareCandidateRow, TraceCompareChurn, TraceCompareGuardrails, TraceCompareOutput,
		TraceComparePolicies, TraceComparePolicy, TraceCompareRegressionAttribution,
		TraceCompareStageDelta, TraceCompareStageRow, TraceCompareSummary, TraceCompareTrace,
		TraceCompareTraceRow, TraceCompareVariant,
	},
};

fn decode_trace_replay_candidates(
	rows: Vec<TraceCompareCandidateRow>,
) -> Vec<TraceReplayCandidate> {
	rows.into_iter()
		.map(|row| {
			let decoded =
				serde_json::from_value::<TraceReplayCandidate>(row.candidate_snapshot.clone())
					.ok()
					.filter(|value| value.note_id != Uuid::nil() && value.chunk_id != Uuid::nil());

			decoded.unwrap_or_else(|| TraceReplayCandidate {
				note_id: row.note_id,
				chunk_id: row.chunk_id,
				chunk_index: row.chunk_index,
				snippet: row.snippet,
				retrieval_rank: u32::try_from(row.retrieval_rank).unwrap_or(0),
				retrieval_score: None,
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
	let candidates = decode_trace_replay_candidates(candidate_rows);
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
