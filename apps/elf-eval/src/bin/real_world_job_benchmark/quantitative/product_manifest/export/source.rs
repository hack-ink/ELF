use crate::{
	ExportQuantitativeProductManifestArgs, QuantitativeBenchmarkRow, RealWorldReport, Result, eyre,
};

pub(super) struct ProductExportIdentity<'report> {
	pub(super) row: &'report QuantitativeBenchmarkRow,
	pub(super) source_product: &'report str,
	pub(super) source_adapter_id: &'report str,
	pub(super) product: &'report str,
	pub(super) adapter_id: &'report str,
	pub(super) adapter_name: &'report str,
}

pub(super) fn product_export_identity<'report>(
	report: &'report RealWorldReport,
	args: &'report ExportQuantitativeProductManifestArgs,
) -> Result<ProductExportIdentity<'report>> {
	let source_row =
		report.quantitative_scoreboard.rows.first().ok_or_else(|| {
			eyre::eyre!("{} has no quantitative product row.", args.report.display())
		})?;
	let source_product = source_row.product.as_str();
	let source_adapter_id = source_row.adapter_id.as_str();
	let product = args.product.as_deref().unwrap_or(source_product).trim();
	let adapter_id = args.adapter_id.as_deref().unwrap_or(source_adapter_id).trim();
	let adapter_name =
		args.adapter_name.as_deref().unwrap_or(source_row.adapter_name.as_str()).trim();

	Ok(ProductExportIdentity {
		row: source_row,
		source_product,
		source_adapter_id,
		product,
		adapter_id,
		adapter_name,
	})
}
