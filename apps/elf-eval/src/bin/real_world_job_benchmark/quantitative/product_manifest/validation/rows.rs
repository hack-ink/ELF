use crate::{
	BTreeSet, Path, QuantitativeBenchmarkRow, QuantitativeProductManifest, Result, eyre,
	quantitative::MIN_LEADERBOARD_QUERY_COUNT,
};

pub(super) fn validate_quantitative_product_rows(
	manifest: &QuantitativeProductManifest,
	path: &Path,
	corpus_id: &str,
) -> Result<()> {
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

	Ok(())
}

pub(super) fn validate_quantitative_per_query_rows(
	manifest: &QuantitativeProductManifest,
	path: &Path,
	corpus_id: &str,
) -> Result<()> {
	let row_keys = manifest
		.rows
		.iter()
		.map(|row| (row.product.as_str(), row.adapter_id.as_str()))
		.collect::<BTreeSet<_>>();

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

	Ok(())
}

pub(super) fn validate_ranked_row_evidence(
	manifest: &QuantitativeProductManifest,
	path: &Path,
) -> Result<()> {
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
