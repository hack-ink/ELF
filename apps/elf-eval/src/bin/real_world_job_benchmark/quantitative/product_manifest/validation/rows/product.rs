mod identity;
mod leaderboard;

use crate::{Path, QuantitativeProductManifest, Result};

pub(super) fn validate_quantitative_product_rows(
	manifest: &QuantitativeProductManifest,
	path: &Path,
	corpus_id: &str,
) -> Result<()> {
	for row in &manifest.rows {
		identity::validate_product_row_identity(path, row, corpus_id)?;

		if row.leaderboard_eligible {
			leaderboard::validate_leaderboard_eligible_product_row(path, row)?;
		}
	}

	Ok(())
}
