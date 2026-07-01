use crate::{Path, PathBuf, Result, fs};

pub(super) fn audit_fixture_paths(path: &Path) -> Result<Vec<PathBuf>> {
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
