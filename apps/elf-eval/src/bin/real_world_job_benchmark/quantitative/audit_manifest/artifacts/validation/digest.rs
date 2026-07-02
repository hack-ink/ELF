use crate::{
	Path, QuantitativeAuditArtifact, Result, eyre,
	quantitative::audit_manifest::artifacts::{digest, paths},
};

pub(super) fn validate_audit_artifact_digest(
	path: &Path,
	artifact: &QuantitativeAuditArtifact,
) -> Result<()> {
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

	Ok(())
}
