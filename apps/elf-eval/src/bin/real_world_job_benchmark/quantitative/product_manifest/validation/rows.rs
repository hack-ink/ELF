mod per_query;
mod product;
mod ranking;

use crate::{Path, QuantitativeProductManifest, Result};

pub(super) fn validate_quantitative_product_rows(
	manifest: &QuantitativeProductManifest,
	path: &Path,
	corpus_id: &str,
) -> Result<()> {
	product::validate_quantitative_product_rows(manifest, path, corpus_id)
}

pub(super) fn validate_quantitative_per_query_rows(
	manifest: &QuantitativeProductManifest,
	path: &Path,
	corpus_id: &str,
) -> Result<()> {
	per_query::validate_quantitative_per_query_rows(manifest, path, corpus_id)
}

pub(super) fn validate_ranked_row_evidence(
	manifest: &QuantitativeProductManifest,
	path: &Path,
) -> Result<()> {
	ranking::validate_ranked_row_evidence(manifest, path)
}
