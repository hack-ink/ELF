use std::collections::HashMap;

use crate::app::trace_compare::types::{
	TraceCompareChurn, TraceCompareGuardrails, TraceCompareRegressionAttribution,
	TraceCompareStageDelta,
};

pub(in crate::app::trace_compare) fn build_trace_compare_regression_attribution(
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
