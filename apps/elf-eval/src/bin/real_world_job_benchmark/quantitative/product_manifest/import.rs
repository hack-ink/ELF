use crate::{
	Path, QuantitativeProductManifest, Result, eyre, fs, quantitative::product_manifest::validation,
};

pub(super) fn quantitative_product_manifest(
	path: Option<&Path>,
	corpus_id: &str,
) -> Result<QuantitativeProductManifest> {
	let Some(path) = path else {
		return Ok(QuantitativeProductManifest::default());
	};
	let raw = fs::read_to_string(path)?;
	let mut manifest =
		serde_json::from_str::<QuantitativeProductManifest>(&raw).map_err(|err| {
			eyre::eyre!("Failed to parse quantitative product manifest {}: {err}", path.display())
		})?;

	populate_source_manifest_corpus_ids(&mut manifest);

	validation::validate_quantitative_product_manifest(&manifest, path, corpus_id)?;

	Ok(manifest)
}

fn populate_source_manifest_corpus_ids(manifest: &mut QuantitativeProductManifest) {
	for row in &mut manifest.rows {
		row.source_manifest_corpus_id.get_or_insert_with(|| manifest.corpus_id.clone());
	}
	for row in &mut manifest.per_query_rows {
		row.source_manifest_corpus_id.get_or_insert_with(|| manifest.corpus_id.clone());
	}
}
