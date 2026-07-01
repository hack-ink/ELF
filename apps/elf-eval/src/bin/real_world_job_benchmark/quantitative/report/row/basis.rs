use crate::{
	QuantitativePerQueryRow,
	quantitative::{
		self, metrics,
		report::{QuantitativeReportInput, row::query_counts},
	},
};

pub(super) struct QuantitativeRowBasis {
	pub(super) corpus_id: String,
	pub(super) evidence_class: &'static str,
	pub(super) per_query_rows: Vec<QuantitativePerQueryRow>,
	pub(super) ranking_query_count: usize,
	pub(super) explicit_qrel_query_count: usize,
	pub(super) metric_comparable: bool,
	pub(super) result_state: &'static str,
}

pub(super) fn quantitative_row_basis(input: &QuantitativeReportInput<'_>) -> QuantitativeRowBasis {
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

	QuantitativeRowBasis {
		corpus_id,
		evidence_class,
		per_query_rows,
		ranking_query_count,
		explicit_qrel_query_count: query_counts.explicit_qrel_query_count,
		metric_comparable: ranking_query_count > 0,
		result_state: quantitative::quantitative_result_state(input.summary),
	}
}
