use crate::{Path, QuantitativeAuditArtifact, Result, eyre};

pub(super) fn validate_audit_artifact_fields(
	path: &Path,
	artifact: &QuantitativeAuditArtifact,
) -> Result<()> {
	if artifact.role.trim().is_empty()
		|| artifact.path.trim().is_empty()
		|| artifact.sha256.trim().is_empty()
	{
		return Err(eyre::eyre!(
			"{} has an incomplete quantitative audit artifact.",
			path.display()
		));
	}
	if artifact.sha256.len() != 64 || !artifact.sha256.chars().all(|ch| ch.is_ascii_hexdigit()) {
		return Err(eyre::eyre!(
			"{} artifact {} has invalid sha256 digest {}.",
			path.display(),
			artifact.role,
			artifact.sha256
		));
	}

	Ok(())
}
