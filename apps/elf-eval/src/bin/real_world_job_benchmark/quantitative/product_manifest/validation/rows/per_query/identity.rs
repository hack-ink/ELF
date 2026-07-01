use crate::{BTreeSet, Path, QuantitativePerQueryRow, Result, eyre};

pub(super) fn validate_per_query_row_identity(
	path: &Path,
	row: &QuantitativePerQueryRow,
	row_keys: &BTreeSet<(&str, &str)>,
	corpus_id: &str,
) -> Result<()> {
	validate_complete_per_query_row(path, row)?;
	validate_matching_product_row(path, row, row_keys)?;

	validate_same_corpus_per_query_row(path, row, corpus_id)
}

fn validate_complete_per_query_row(path: &Path, row: &QuantitativePerQueryRow) -> Result<()> {
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

	Ok(())
}

fn validate_matching_product_row(
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

fn validate_same_corpus_per_query_row(
	path: &Path,
	row: &QuantitativePerQueryRow,
	corpus_id: &str,
) -> Result<()> {
	if row.source_manifest_corpus_id.as_deref() != Some(corpus_id) {
		return Err(eyre::eyre!(
			"{} per-query row {}:{} is not same-corpus {}.",
			path.display(),
			row.product,
			row.adapter_id,
			corpus_id
		));
	}

	Ok(())
}
