mod validation;

use crate::{
	ExportQuantitativeProductManifestArgs, Path, QuantitativeProductManifest, REPORT_SCHEMA,
	RealWorldReport, Result, eyre, fs, quantitative::QUANTITATIVE_PRODUCT_MANIFEST_SCHEMA,
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

	if product.is_empty() || adapter_id.is_empty() || adapter_name.is_empty() {
		return Err(eyre::eyre!(
			"{} cannot export an incomplete quantitative product identity.",
			args.report.display()
		));
	}
	if product == "ELF" {
		return Err(eyre::eyre!(
			"{} exports product ELF; use --product for external product manifest exports.",
			args.report.display()
		));
	}

	let mut row = source_row.clone();

	row.product = product.to_string();
	row.adapter_id = adapter_id.to_string();
	row.adapter_name = adapter_name.to_string();
	row.claim_boundary = concat!(
		"Exported from a generated real_world_job_report quantitative row; ",
		"import remains subject to same-corpus, per-query, explicit-qrel, and leaderboard gates."
	)
	.to_string();

	let mut per_query_rows = Vec::new();

	for row in &report.quantitative_scoreboard.per_query_rows {
		if row.product != source_product || row.adapter_id != source_adapter_id {
			continue;
		}

		let mut row = row.clone();

		row.product = product.to_string();
		row.adapter_id = adapter_id.to_string();
		row.claim_boundary = concat!(
			"Exported from generated report per-query quantitative evidence; ",
			"import does not relax paired-significance or leaderboard gates."
		)
		.to_string();

		per_query_rows.push(row);
	}

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

pub(super) fn quantitative_product_manifest(
	path: Option<&Path>,
	corpus_id: &str,
) -> Result<QuantitativeProductManifest> {
	let Some(path) = path else {
		return Ok(QuantitativeProductManifest::default());
	};
	let raw = fs::read_to_string(path)?;
	let mut manifest =
		serde_json::from_str::<QuantitativeProductManifest>(&raw).map_err(|err| {
			eyre::eyre!("Failed to parse quantitative product manifest {}: {err}", path.display())
		})?;

	for row in &mut manifest.rows {
		row.source_manifest_corpus_id.get_or_insert_with(|| manifest.corpus_id.clone());
	}
	for row in &mut manifest.per_query_rows {
		row.source_manifest_corpus_id.get_or_insert_with(|| manifest.corpus_id.clone());
	}

	validation::validate_quantitative_product_manifest(&manifest, path, corpus_id)?;

	Ok(manifest)
}
