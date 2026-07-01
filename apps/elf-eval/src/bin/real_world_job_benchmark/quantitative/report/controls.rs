use crate::{
	QuantitativeBenchmarkControls,
	quantitative::{MIN_LEADERBOARD_QUERY_COUNT, report::QuantitativeReportInput},
};

pub(super) fn quantitative_benchmark_controls(
	input: &QuantitativeReportInput<'_>,
	ranking_query_count: usize,
	explicit_qrel_query_count: usize,
	leaderboard_claim_allowed: bool,
) -> QuantitativeBenchmarkControls {
	QuantitativeBenchmarkControls {
		same_corpus_required: true,
		same_task_required: true,
		ranked_candidates_required_for_ranking_metrics: true,
		explicit_relevance_judgments_required_for_leaderboard: true,
		minimum_query_count_for_leaderboard: MIN_LEADERBOARD_QUERY_COUNT,
		current_query_count: input.source_jobs.len(),
		current_ranking_query_count: ranking_query_count,
		current_explicit_qrel_query_count: explicit_qrel_query_count,
		leaderboard_claim_allowed,
		leakage_control:
			"held_out_or_leakage_audited_runtime_rows_required_before_leaderboard_claims"
				.to_string(),
	}
}
