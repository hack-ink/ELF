use crate::{BTreeSet, Path, QuantitativePerQueryRow, Result, eyre};

pub(super) fn validate_matching_product_row(
	path: &Path,
	row: &QuantitativePerQueryRow,
	row_keys: &BTreeSet<(&str, &str)>,
) -> Result<()> {
	if !row_keys.contains(&(row.product.as_str(), row.adapter_id.as_str())) {
		return Err(eyre::eyre!(
			"{} per-query row {}:{} has no matching product row.",
			path.display(),
			row.product,
			row.adapter_id
		));
	}

	Ok(())
}
