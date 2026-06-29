use crate::app::{
	metrics,
	types::{
		CompareQueryReport, EvalSummary, EvalSummaryDelta, PolicyChurn, PolicyStabilitySummary,
		QueryReport, QueryStabilityDelta, QueryVariantDelta, QueryVariantReport,
		StabilitySummaryDelta,
	},
};

pub(super) fn diff_summary(a: &EvalSummary, b: &EvalSummary) -> EvalSummaryDelta {
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

pub(super) fn build_compare_queries(
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
			let (positional_churn_at_k, set_churn_at_k) = metrics::churn_against_baseline_at_k(
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
