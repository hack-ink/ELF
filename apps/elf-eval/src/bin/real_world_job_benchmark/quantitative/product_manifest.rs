mod export;
mod import;
mod validation;

pub(crate) use self::export::quantitative_product_manifest_from_report;

use crate::{Path, QuantitativeProductManifest, Result};

pub(super) fn quantitative_product_manifest(
	path: Option<&Path>,
	corpus_id: &str,
) -> Result<QuantitativeProductManifest> {
	import::quantitative_product_manifest(path, corpus_id)
}
