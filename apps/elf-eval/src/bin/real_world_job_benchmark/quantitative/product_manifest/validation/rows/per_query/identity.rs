mod corpus;
mod fields;
mod product;

use crate::{BTreeSet, Path, QuantitativePerQueryRow, Result};

pub(super) fn validate_per_query_row_identity(
	path: &Path,
	row: &QuantitativePerQueryRow,
	row_keys: &BTreeSet<(&str, &str)>,
	corpus_id: &str,
) -> Result<()> {
	fields::validate_complete_per_query_row(path, row)?;
	product::validate_matching_product_row(path, row, row_keys)?;

	corpus::validate_same_corpus_per_query_row(path, row, corpus_id)
}
