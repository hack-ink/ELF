use crate::{Path, PathBuf, Result, fs};

pub(in crate::quantitative::audit_manifest) fn fixture_path_digest(path: &Path) -> Result<String> {
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
