use crate::{BTreeSet, Path, QuantitativeProductManifest, Result, eyre};

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
