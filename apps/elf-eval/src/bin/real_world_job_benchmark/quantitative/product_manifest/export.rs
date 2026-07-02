mod identity;
mod manifest;
mod rows;
mod source;

use crate::{
	ExportQuantitativeProductManifestArgs, QuantitativeProductManifest, REPORT_SCHEMA,
	RealWorldReport, Result, eyre, quantitative::product_manifest::validation,
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

	let manifest = manifest::quantitative_product_manifest(report, args)?;

	validation::validate_quantitative_product_manifest(
		&manifest,
		&args.report,
		manifest.corpus_id.as_str(),
	)?;

	Ok(manifest)
}
