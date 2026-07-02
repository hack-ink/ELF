use crate::{
	ExportQuantitativeProductManifestArgs, QuantitativeProductManifest, RealWorldReport, Result,
	quantitative::{
		QUANTITATIVE_PRODUCT_MANIFEST_SCHEMA,
		product_manifest::export::{identity, rows, source},
	},
};

pub(super) fn quantitative_product_manifest(
	report: &RealWorldReport,
	args: &ExportQuantitativeProductManifestArgs,
) -> Result<QuantitativeProductManifest> {
	let source = source::product_export_identity(report, args)?;

	identity::validate_export_identity(
		args,
		source.product,
		source.adapter_id,
		source.adapter_name,
	)?;

	let row = rows::exported_product_row(
		source.row,
		source.product,
		source.adapter_id,
		source.adapter_name,
	);
	let per_query_rows = rows::exported_per_query_rows(
		report,
		source.source_product,
		source.source_adapter_id,
		source.product,
		source.adapter_id,
	);

	Ok(QuantitativeProductManifest {
		schema: QUANTITATIVE_PRODUCT_MANIFEST_SCHEMA.to_string(),
		manifest_id: args
			.manifest_id
			.clone()
			.unwrap_or_else(|| format!("{}-quantitative-product-manifest", report.run_id)),
		corpus_id: report.quantitative_scoreboard.corpus_id.clone(),
		rows: vec![row],
		per_query_rows,
	})
}
