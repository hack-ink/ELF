use std::env;

use crate::{Path, PathBuf, QuantitativeAuditManifest, Result, eyre, fs};

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

		let artifact_path = resolve_quantitative_audit_artifact_path(path, artifact.path.as_str());
		let actual = fixture_path_digest(artifact_path.as_path()).map_err(|err| {
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

pub(super) fn fixture_path_digest(path: &Path) -> Result<String> {
	let mut hasher = blake3::Hasher::new();

	if path.is_file() {
		hash_fixture_file(
			path,
			path.file_name().and_then(|name| name.to_str()).unwrap_or("fixture"),
			&mut hasher,
		)?;

		return Ok(hasher.finalize().to_hex().to_string());
	}

	let paths = audit_fixture_paths(path)?;

	for fixture in paths {
		let relative = fixture
			.strip_prefix(path)
			.map(|relative| relative.to_string_lossy().replace('\\', "/"))
			.unwrap_or_else(|_| fixture.to_string_lossy().replace('\\', "/"));

		hash_fixture_file(fixture.as_path(), relative.as_str(), &mut hasher)?;
	}

	Ok(hasher.finalize().to_hex().to_string())
}

pub(super) fn audit_artifact_display_path(path: &Path) -> String {
	let display_path = if path.is_absolute() {
		env::current_dir()
			.ok()
			.and_then(|cwd| path.strip_prefix(cwd).ok().map(Path::to_path_buf))
			.unwrap_or_else(|| path.to_path_buf())
	} else {
		path.to_path_buf()
	};

	display_path.to_string_lossy().replace('\\', "/")
}

fn resolve_quantitative_audit_artifact_path(manifest_path: &Path, artifact_path: &str) -> PathBuf {
	let raw = PathBuf::from(artifact_path);

	if raw.is_absolute() {
		return raw;
	}

	let cwd_path = env::current_dir().map(|cwd| cwd.join(&raw)).unwrap_or_else(|_| raw.clone());

	if cwd_path.exists() {
		return cwd_path;
	}

	manifest_path.parent().map(|parent| parent.join(&raw)).unwrap_or(cwd_path)
}

fn audit_fixture_paths(path: &Path) -> Result<Vec<PathBuf>> {
	let mut paths = Vec::new();

	collect_audit_fixture_paths(path, &mut paths)?;

	paths.sort();

	Ok(paths)
}

fn collect_audit_fixture_paths(path: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
	if path.is_file() {
		paths.push(path.to_path_buf());

		return Ok(());
	}

	for entry in fs::read_dir(path)? {
		let entry_path = entry?.path();

		if entry_path.is_dir() {
			collect_audit_fixture_paths(entry_path.as_path(), paths)?;
		} else if entry_path.extension().and_then(|ext| ext.to_str()) == Some("json") {
			paths.push(entry_path);
		}
	}

	Ok(())
}

fn hash_fixture_file(path: &Path, logical_path: &str, hasher: &mut blake3::Hasher) -> Result<()> {
	hasher.update(logical_path.as_bytes());
	hasher.update(b"\0");
	hasher.update(&fs::read(path)?);
	hasher.update(b"\0");

	Ok(())
}
