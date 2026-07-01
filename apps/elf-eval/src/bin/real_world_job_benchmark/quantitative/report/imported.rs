use crate::{
	Path, QuantitativeBenchmarkRow, QuantitativePerQueryRow, Result, quantitative::product_manifest,
};

pub(super) struct ImportedQuantitativeRows {
	pub(super) rows: Vec<QuantitativeBenchmarkRow>,
	pub(super) per_query_rows: Vec<QuantitativePerQueryRow>,
	pub(super) row_count: usize,
	pub(super) per_query_count: usize,
}

pub(super) fn imported_quantitative_rows(
	product_manifest_path: Option<&Path>,
	corpus_id: &str,
) -> Result<ImportedQuantitativeRows> {
	let product_manifest =
		product_manifest::quantitative_product_manifest(product_manifest_path, corpus_id)?;
	let row_count = product_manifest.rows.len();
	let per_query_count = product_manifest.per_query_rows.len();

	Ok(ImportedQuantitativeRows {
		rows: product_manifest.rows,
		per_query_rows: product_manifest.per_query_rows,
		row_count,
		per_query_count,
	})
}
