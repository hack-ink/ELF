use crate::{Path, QuantitativeProductManifest, Result, eyre};

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
