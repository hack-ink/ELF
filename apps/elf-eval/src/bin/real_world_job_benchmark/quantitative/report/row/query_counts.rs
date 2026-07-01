use crate::QuantitativePerQueryRow;

pub(super) struct QuantitativeQueryCounts {
	pub(super) ranking_query_count: usize,
	pub(super) explicit_qrel_query_count: usize,
}

pub(super) fn quantitative_query_counts(
	per_query_rows: &[QuantitativePerQueryRow],
) -> QuantitativeQueryCounts {
	QuantitativeQueryCounts {
		ranking_query_count: per_query_rows
			.iter()
			.filter(|row| row.candidate_count > 0 && row.expected_relevant_count > 0)
			.count(),
		explicit_qrel_query_count: per_query_rows
			.iter()
			.filter(|row| row.qrel_source == "explicit_qrels")
			.count(),
	}
}
