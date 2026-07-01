mod paths;

use crate::{Path, Result, fs};

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

	let paths = paths::audit_fixture_paths(path)?;

	for fixture in paths {
		let relative = fixture
			.strip_prefix(path)
			.map(|relative| relative.to_string_lossy().replace('\\', "/"))
			.unwrap_or_else(|_| fixture.to_string_lossy().replace('\\', "/"));

		hash_fixture_file(fixture.as_path(), relative.as_str(), &mut hasher)?;
	}

	Ok(hasher.finalize().to_hex().to_string())
}

fn hash_fixture_file(path: &Path, logical_path: &str, hasher: &mut blake3::Hasher) -> Result<()> {
	hasher.update(logical_path.as_bytes());
	hasher.update(b"\0");
	hasher.update(&fs::read(path)?);
	hasher.update(b"\0");

	Ok(())
}
