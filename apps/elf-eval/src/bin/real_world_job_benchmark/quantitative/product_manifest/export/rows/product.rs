use crate::QuantitativeBenchmarkRow;

pub(in crate::quantitative::product_manifest::export) fn exported_product_row(
	source_row: &QuantitativeBenchmarkRow,
	product: &str,
	adapter_id: &str,
	adapter_name: &str,
) -> QuantitativeBenchmarkRow {
	let mut row = source_row.clone();

	row.product = product.to_string();
	row.adapter_id = adapter_id.to_string();
	row.adapter_name = adapter_name.to_string();
	row.claim_boundary = concat!(
		"Exported from a generated real_world_job_report quantitative row; ",
		"import remains subject to same-corpus, per-query, explicit-qrel, and leaderboard gates."
	)
	.to_string();

	row
}
