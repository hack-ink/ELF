use crate::{
	BTreeSet, ExportQuantitativeProductManifestArgs, Path, QuantitativeBenchmarkRow,
	QuantitativeProductManifest, REPORT_SCHEMA, RealWorldReport, Result, eyre, fs,
};

use super::{MIN_LEADERBOARD_QUERY_COUNT, QUANTITATIVE_PRODUCT_MANIFEST_SCHEMA};

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

	validate_quantitative_product_manifest(&manifest, &args.report, manifest.corpus_id.as_str())?;

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

	validate_quantitative_product_manifest(&manifest, path, corpus_id)?;

	Ok(manifest)
}

fn validate_quantitative_product_manifest(
	manifest: &QuantitativeProductManifest,
	path: &Path,
	corpus_id: &str,
) -> Result<()> {
	if manifest.schema != QUANTITATIVE_PRODUCT_MANIFEST_SCHEMA {
		return Err(eyre::eyre!(
			"{} has schema {}, expected {QUANTITATIVE_PRODUCT_MANIFEST_SCHEMA}.",
			path.display(),
			manifest.schema
		));
	}
	if manifest.manifest_id.trim().is_empty() {
		return Err(eyre::eyre!("{} has an empty manifest_id.", path.display()));
	}
	if manifest.corpus_id != corpus_id {
		return Err(eyre::eyre!(
			"{} has corpus_id {}, expected same-corpus {}.",
			path.display(),
			manifest.corpus_id,
			corpus_id
		));
	}
	if manifest.rows.is_empty() {
		return Err(eyre::eyre!("{} declares no quantitative product rows.", path.display()));
	}

	let row_keys = manifest
		.rows
		.iter()
		.map(|row| (row.product.as_str(), row.adapter_id.as_str()))
		.collect::<BTreeSet<_>>();

	for row in &manifest.rows {
		if row.product == "ELF" {
			return Err(eyre::eyre!(
				"{} quantitative product manifest must not inject ELF self rows.",
				path.display()
			));
		}
		if row.product.trim().is_empty()
			|| row.adapter_id.trim().is_empty()
			|| row.adapter_name.trim().is_empty()
			|| row.suite.trim().is_empty()
			|| row.evidence_class.trim().is_empty()
			|| row.result_state.trim().is_empty()
		{
			return Err(eyre::eyre!(
				"{} has an incomplete quantitative product row.",
				path.display()
			));
		}
		if row.source_manifest_corpus_id.as_deref() != Some(corpus_id) {
			return Err(eyre::eyre!(
				"{} row {}:{} is not same-corpus {}.",
				path.display(),
				row.product,
				row.adapter_id,
				corpus_id
			));
		}
		if row.leaderboard_eligible {
			validate_leaderboard_eligible_product_row(path, row)?;
		}
	}
	for row in &manifest.per_query_rows {
		if row.job_id.trim().is_empty()
			|| row.suite.trim().is_empty()
			|| row.evidence_class.trim().is_empty()
			|| row.result_state.trim().is_empty()
			|| row.product.trim().is_empty()
			|| row.adapter_id.trim().is_empty()
			|| row.qrel_source.trim().is_empty()
		{
			return Err(eyre::eyre!(
				"{} has an incomplete quantitative per-query product row.",
				path.display()
			));
		}
		if !row_keys.contains(&(row.product.as_str(), row.adapter_id.as_str())) {
			return Err(eyre::eyre!(
				"{} per-query row {}:{} has no matching product row.",
				path.display(),
				row.product,
				row.adapter_id
			));
		}
		if row.source_manifest_corpus_id.as_deref() != Some(corpus_id) {
			return Err(eyre::eyre!(
				"{} per-query row {}:{} is not same-corpus {}.",
				path.display(),
				row.product,
				row.adapter_id,
				corpus_id
			));
		}
	}
	for row in &manifest.rows {
		if row.ranking_query_count == 0 {
			continue;
		}

		let per_query_count = manifest
			.per_query_rows
			.iter()
			.filter(|per_query| {
				per_query.product == row.product && per_query.adapter_id == row.adapter_id
			})
			.count();

		if per_query_count < row.ranking_query_count {
			return Err(eyre::eyre!(
				"{} row {}:{} declares {} ranked queries but only {} per-query rows.",
				path.display(),
				row.product,
				row.adapter_id,
				row.ranking_query_count,
				per_query_count
			));
		}
	}

	Ok(())
}

fn validate_leaderboard_eligible_product_row(
	path: &Path,
	row: &QuantitativeBenchmarkRow,
) -> Result<()> {
	let has_audit_manifest_id = row
		.audit_manifest_id
		.as_deref()
		.is_some_and(|audit_manifest_id| !audit_manifest_id.trim().is_empty());

	if row.evidence_class != "live_real_world"
		|| row.sample_size < MIN_LEADERBOARD_QUERY_COUNT
		|| row.ranking_query_count != row.sample_size
		|| row.explicit_qrel_query_count != row.ranking_query_count
		|| !row.held_out
		|| !row.leakage_audited
		|| !has_audit_manifest_id
	{
		return Err(eyre::eyre!(
			"{} row {}:{} is marked leaderboard_eligible without the required live/product-runtime, query-count, explicit-qrel, held-out, leakage-audit, and audit-manifest controls.",
			path.display(),
			row.product,
			row.adapter_id
		));
	}

	Ok(())
}
