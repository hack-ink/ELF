use crate::{QuantitativePerQueryRow, RealWorldReport};

pub(in crate::quantitative::product_manifest::export) fn exported_per_query_rows(
	report: &RealWorldReport,
	source_product: &str,
	source_adapter_id: &str,
	product: &str,
	adapter_id: &str,
) -> Vec<QuantitativePerQueryRow> {
	report
		.quantitative_scoreboard
		.per_query_rows
		.iter()
		.filter(|row| row.product == source_product && row.adapter_id == source_adapter_id)
		.map(|row| exported_per_query_row(row, product, adapter_id))
		.collect()
}

fn exported_per_query_row(
	source_row: &QuantitativePerQueryRow,
	product: &str,
	adapter_id: &str,
) -> QuantitativePerQueryRow {
	let mut row = source_row.clone();

	row.product = product.to_string();
	row.adapter_id = adapter_id.to_string();
	row.claim_boundary = concat!(
		"Exported from generated report per-query quantitative evidence; ",
		"import does not relax paired-significance or leaderboard gates."
	)
	.to_string();

	row
}
