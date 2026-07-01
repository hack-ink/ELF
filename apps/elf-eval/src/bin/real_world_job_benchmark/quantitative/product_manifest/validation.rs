mod rows;

use crate::{
	Path, QuantitativeProductManifest, Result, eyre,
	quantitative::QUANTITATIVE_PRODUCT_MANIFEST_SCHEMA,
};

pub(super) fn validate_quantitative_product_manifest(
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

	rows::validate_quantitative_product_rows(manifest, path, corpus_id)?;
	rows::validate_quantitative_per_query_rows(manifest, path, corpus_id)?;
	rows::validate_ranked_row_evidence(manifest, path)?;

	Ok(())
}
