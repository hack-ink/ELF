mod query_counts;

use crate::{
	QuantitativeBenchmarkRow, QuantitativePerQueryRow, Result,
	quantitative::{
		self, QUANTITATIVE_ROW_CLAIM_BOUNDARY,
		audit_manifest::{self, QuantitativeAuditContext},
		metrics,
		report::QuantitativeReportInput,
	},
};

pub(super) struct CurrentQuantitativeRow {
	pub(super) corpus_id: String,
	pub(super) row: QuantitativeBenchmarkRow,
	pub(super) per_query_rows: Vec<QuantitativePerQueryRow>,
	pub(super) ranking_query_count: usize,
	pub(super) explicit_qrel_query_count: usize,
}

pub(super) fn current_quantitative_row(
	input: &QuantitativeReportInput<'_>,
) -> Result<CurrentQuantitativeRow> {
	let corpus_id = quantitative::quantitative_corpus_id(input.source_jobs);
	let evidence_class = quantitative::quantitative_evidence_class(input.adapter, input.jobs);
	let per_query_rows = metrics::quantitative_per_query_rows(
		input.source_jobs,
		input.jobs,
		corpus_id.as_str(),
		evidence_class,
		input.adapter.adapter_id.as_str(),
	);
	let query_counts = query_counts::quantitative_query_counts(per_query_rows.as_slice());
	let ranking_query_count = query_counts.ranking_query_count;
	let explicit_qrel_query_count = query_counts.explicit_qrel_query_count;
	let metric_comparable = ranking_query_count > 0;
	let result_state = quantitative::quantitative_result_state(input.summary);
	let audit_evidence = audit_manifest::quantitative_audit_evidence(
		input.audit_manifest_path,
		QuantitativeAuditContext {
			run_id: input.run_id,
			corpus_id: corpus_id.as_str(),
			product: "ELF",
			adapter_id: input.adapter.adapter_id.as_str(),
			source_jobs: input.source_jobs,
			ranking_query_count,
			explicit_qrel_query_count,
		},
	)?;
	let leaderboard_eligible = quantitative::quantitative_row_leaderboard_eligible(
		evidence_class,
		input.source_jobs.len(),
		ranking_query_count,
		explicit_qrel_query_count,
		metric_comparable,
		&audit_evidence,
	);
	let row = QuantitativeBenchmarkRow {
		product: "ELF".to_string(),
		adapter_id: input.adapter.adapter_id.clone(),
		adapter_name: input.adapter.name.clone(),
		suite: quantitative::quantitative_suite_id(input.jobs),
		evidence_class: evidence_class.to_string(),
		source_manifest_corpus_id: Some(corpus_id.clone()),
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
		metrics: metrics::aggregate_metrics(per_query_rows.as_slice()),
		metric_states: metrics::aggregate_metric_states(result_state, metric_comparable),
		denominators: metrics::aggregate_denominators(per_query_rows.as_slice()),
		confidence_intervals: metrics::aggregate_confidence_intervals(per_query_rows.as_slice()),
		claim_boundary: QUANTITATIVE_ROW_CLAIM_BOUNDARY.to_string(),
	};

	Ok(CurrentQuantitativeRow {
		corpus_id,
		row,
		per_query_rows,
		ranking_query_count,
		explicit_qrel_query_count,
	})
}
