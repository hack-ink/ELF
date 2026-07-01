use crate::{
	QuantitativeBenchmarkRow, QuantitativePerQueryRow,
	quantitative::{
		self, QUANTITATIVE_ROW_CLAIM_BOUNDARY, audit_manifest::QuantitativeAuditEvidence, metrics,
		report::QuantitativeReportInput,
	},
};

pub(super) struct QuantitativeBenchmarkRowInput<'a, 'b> {
	pub(super) input: &'a QuantitativeReportInput<'b>,
	pub(super) corpus_id: &'a str,
	pub(super) evidence_class: &'a str,
	pub(super) per_query_rows: &'a [QuantitativePerQueryRow],
	pub(super) ranking_query_count: usize,
	pub(super) explicit_qrel_query_count: usize,
	pub(super) metric_comparable: bool,
	pub(super) result_state: &'a str,
	pub(super) audit_evidence: QuantitativeAuditEvidence,
	pub(super) leaderboard_eligible: bool,
}

pub(super) fn quantitative_benchmark_row(
	row_input: QuantitativeBenchmarkRowInput<'_, '_>,
) -> QuantitativeBenchmarkRow {
	let QuantitativeBenchmarkRowInput {
		input,
		corpus_id,
		evidence_class,
		per_query_rows,
		ranking_query_count,
		explicit_qrel_query_count,
		metric_comparable,
		result_state,
		audit_evidence,
		leaderboard_eligible,
	} = row_input;

	QuantitativeBenchmarkRow {
		product: "ELF".to_string(),
		adapter_id: input.adapter.adapter_id.clone(),
		adapter_name: input.adapter.name.clone(),
		suite: quantitative::quantitative_suite_id(input.jobs),
		evidence_class: evidence_class.to_string(),
		source_manifest_corpus_id: Some(corpus_id.to_string()),
		result_state: result_state.to_string(),
		comparable: metric_comparable,
		metric_comparable,
		leaderboard_eligible,
		held_out: audit_evidence.held_out,
		leakage_audited: audit_evidence.leakage_audited,
		audit_manifest_id: audit_evidence.audit_manifest_id,
		fixture_regression_only: evidence_class == "fixture_backed",
		sample_size: input.jobs.len(),
		ranking_query_count,
		ranking_coverage_state: metrics::ranking_coverage_state(
			input.summary,
			input.source_jobs.len(),
			ranking_query_count,
		)
		.to_string(),
		ranked_candidate_source: metrics::ranked_candidate_source(ranking_query_count).to_string(),
		qrel_source: metrics::aggregate_qrel_source(ranking_query_count, explicit_qrel_query_count)
			.to_string(),
		explicit_qrel_query_count,
		metrics: metrics::aggregate_metrics(per_query_rows),
		metric_states: metrics::aggregate_metric_states(result_state, metric_comparable),
		denominators: metrics::aggregate_denominators(per_query_rows),
		confidence_intervals: metrics::aggregate_confidence_intervals(per_query_rows),
		claim_boundary: QUANTITATIVE_ROW_CLAIM_BOUNDARY.to_string(),
	}
}
