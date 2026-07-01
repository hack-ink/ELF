mod audit_gates;
mod benchmark_row;
mod query_counts;

use crate::{
	QuantitativeBenchmarkRow, QuantitativePerQueryRow, Result,
	quantitative::{
		self, metrics,
		report::{QuantitativeReportInput, row::benchmark_row::QuantitativeBenchmarkRowInput},
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
	let audit_gates = audit_gates::quantitative_audit_gates(
		input,
		corpus_id.as_str(),
		evidence_class,
		ranking_query_count,
		explicit_qrel_query_count,
		metric_comparable,
	)?;
	let row = benchmark_row::quantitative_benchmark_row(QuantitativeBenchmarkRowInput {
		input,
		corpus_id: corpus_id.as_str(),
		evidence_class,
		per_query_rows: per_query_rows.as_slice(),
		ranking_query_count,
		explicit_qrel_query_count,
		metric_comparable,
		result_state,
		audit_evidence: audit_gates.audit_evidence,
		leaderboard_eligible: audit_gates.leaderboard_eligible,
	});

	Ok(CurrentQuantitativeRow {
		corpus_id,
		row,
		per_query_rows,
		ranking_query_count,
		explicit_qrel_query_count,
	})
}
