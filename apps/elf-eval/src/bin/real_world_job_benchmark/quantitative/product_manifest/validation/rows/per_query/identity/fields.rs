use crate::{Path, QuantitativePerQueryRow, Result, eyre};

pub(super) fn validate_complete_per_query_row(
	path: &Path,
	row: &QuantitativePerQueryRow,
) -> Result<()> {
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
