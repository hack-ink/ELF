use crate::{
	Path, QuantitativeAuditManifest, Result, eyre, quantitative::QUANTITATIVE_AUDIT_MANIFEST_SCHEMA,
};

pub(super) fn validate_quantitative_audit_schema(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
) -> Result<()> {
	if manifest.schema != QUANTITATIVE_AUDIT_MANIFEST_SCHEMA {
		return Err(eyre::eyre!(
			"{} has schema {}, expected {QUANTITATIVE_AUDIT_MANIFEST_SCHEMA}.",
			path.display(),
			manifest.schema
		));
	}
	if manifest.manifest_id.trim().is_empty() {
		return Err(eyre::eyre!("{} has an empty manifest_id.", path.display()));
	}

	Ok(())
}
