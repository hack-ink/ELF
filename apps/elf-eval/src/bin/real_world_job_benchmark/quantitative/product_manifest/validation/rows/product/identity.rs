use crate::{Path, QuantitativeBenchmarkRow, Result, eyre};

pub(super) fn validate_product_row_identity(
	path: &Path,
	row: &QuantitativeBenchmarkRow,
	corpus_id: &str,
) -> Result<()> {
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
		return Err(eyre::eyre!("{} has an incomplete quantitative product row.", path.display()));
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

	Ok(())
}
