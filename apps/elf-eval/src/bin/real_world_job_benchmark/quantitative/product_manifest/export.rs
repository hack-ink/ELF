mod identity;
mod rows;

use crate::{
	ExportQuantitativeProductManifestArgs, QuantitativeProductManifest, REPORT_SCHEMA,
	RealWorldReport, Result, eyre,
	quantitative::{QUANTITATIVE_PRODUCT_MANIFEST_SCHEMA, product_manifest::validation},
};

pub(crate) fn quantitative_product_manifest_from_report(
	report: &RealWorldReport,
	args: &ExportQuantitativeProductManifestArgs,
) -> Result<QuantitativeProductManifest> {
	if report.schema != REPORT_SCHEMA {
		return Err(eyre::eyre!(
			"{} has schema {}, expected {REPORT_SCHEMA}.",
			args.report.display(),
			report.schema
		));
	}

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

	identity::validate_export_identity(args, product, adapter_id, adapter_name)?;

	let row = rows::exported_product_row(source_row, product, adapter_id, adapter_name);
	let per_query_rows = rows::exported_per_query_rows(
		report,
		source_product,
		source_adapter_id,
		product,
		adapter_id,
	);
	let manifest = QuantitativeProductManifest {
		schema: QUANTITATIVE_PRODUCT_MANIFEST_SCHEMA.to_string(),
		manifest_id: args
			.manifest_id
			.clone()
			.unwrap_or_else(|| format!("{}-quantitative-product-manifest", report.run_id)),
		corpus_id: report.quantitative_scoreboard.corpus_id.clone(),
		rows: vec![row],
		per_query_rows,
	};

	validation::validate_quantitative_product_manifest(
		&manifest,
		&args.report,
		manifest.corpus_id.as_str(),
	)?;

	Ok(manifest)
}
