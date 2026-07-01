mod audit_gates;
mod basis;
mod benchmark_row;
mod query_counts;

use crate::{
	QuantitativeBenchmarkRow, QuantitativePerQueryRow, Result,
	quantitative::report::{
		QuantitativeReportInput, row::benchmark_row::QuantitativeBenchmarkRowInput,
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
	let basis = basis::quantitative_row_basis(input);
	let audit_gates = audit_gates::quantitative_audit_gates(
		input,
		basis.corpus_id.as_str(),
		basis.evidence_class,
		basis.ranking_query_count,
		basis.explicit_qrel_query_count,
		basis.metric_comparable,
	)?;
	let row = benchmark_row::quantitative_benchmark_row(QuantitativeBenchmarkRowInput {
		input,
		corpus_id: basis.corpus_id.as_str(),
		evidence_class: basis.evidence_class,
		per_query_rows: basis.per_query_rows.as_slice(),
		ranking_query_count: basis.ranking_query_count,
		explicit_qrel_query_count: basis.explicit_qrel_query_count,
		metric_comparable: basis.metric_comparable,
		result_state: basis.result_state,
		audit_evidence: audit_gates.audit_evidence,
		leaderboard_eligible: audit_gates.leaderboard_eligible,
	});

	Ok(CurrentQuantitativeRow {
		corpus_id: basis.corpus_id,
		row,
		per_query_rows: basis.per_query_rows,
		ranking_query_count: basis.ranking_query_count,
		explicit_qrel_query_count: basis.explicit_qrel_query_count,
	})
}
