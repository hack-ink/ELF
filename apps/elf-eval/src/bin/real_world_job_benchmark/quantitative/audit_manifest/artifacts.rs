mod digest;
mod paths;

pub(super) use self::{digest::fixture_path_digest, paths::audit_artifact_display_path};

use crate::{Path, QuantitativeAuditManifest, Result, eyre};

pub(super) fn validate_quantitative_audit_artifacts(
	manifest: &QuantitativeAuditManifest,
	path: &Path,
) -> Result<()> {
	if manifest.artifacts.is_empty() {
		return Err(eyre::eyre!("{} has no quantitative audit artifacts.", path.display()));
	}

	for artifact in &manifest.artifacts {
		if artifact.role.trim().is_empty()
			|| artifact.path.trim().is_empty()
			|| artifact.sha256.trim().is_empty()
		{
			return Err(eyre::eyre!(
				"{} has an incomplete quantitative audit artifact.",
				path.display()
			));
		}
		if artifact.sha256.len() != 64 || !artifact.sha256.chars().all(|ch| ch.is_ascii_hexdigit())
		{
			return Err(eyre::eyre!(
				"{} artifact {} has invalid sha256 digest {}.",
				path.display(),
				artifact.role,
				artifact.sha256
			));
		}

		let artifact_path =
			paths::resolve_quantitative_audit_artifact_path(path, artifact.path.as_str());
		let actual = digest::fixture_path_digest(artifact_path.as_path()).map_err(|err| {
			eyre::eyre!(
				"{} artifact {} could not be digested at {}: {err}",
				path.display(),
				artifact.role,
				artifact_path.display()
			)
		})?;

		if actual != artifact.sha256 {
			return Err(eyre::eyre!(
				"{} artifact {} sha256 mismatch for {}: manifest {}, actual {}.",
				path.display(),
				artifact.role,
				artifact_path.display(),
				artifact.sha256,
				actual
			));
		}
	}

	Ok(())
}
