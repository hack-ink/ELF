use crate::{Path, QuantitativePerQueryRow, Result, eyre};

pub(super) fn validate_same_corpus_per_query_row(
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
