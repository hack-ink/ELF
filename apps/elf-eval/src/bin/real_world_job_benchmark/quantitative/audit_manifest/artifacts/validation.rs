mod digest;
mod fields;

use crate::{Path, QuantitativeAuditManifest, Result, eyre};

pub(in crate::quantitative::audit_manifest) fn validate_quantitative_audit_artifacts(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
) -> Result<()> {
	if manifest.artifacts.is_empty() {
		return Err(eyre::eyre!("{} has no quantitative audit artifacts.", path.display()));
	}

	for artifact in &manifest.artifacts {
		fields::validate_audit_artifact_fields(path, artifact)?;
		digest::validate_audit_artifact_digest(path, artifact)?;
	}

	Ok(())
}
