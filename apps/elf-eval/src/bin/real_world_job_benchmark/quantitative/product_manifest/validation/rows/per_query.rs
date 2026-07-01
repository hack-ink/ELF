mod identity;

use crate::{BTreeSet, Path, QuantitativeProductManifest, Result};

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
		identity::validate_per_query_row_identity(path, row, &row_keys, corpus_id)?;
	}

	Ok(())
}
