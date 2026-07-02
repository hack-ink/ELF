use std::env;

use crate::{Path, PathBuf};

pub(in crate::quantitative::audit_manifest) fn audit_artifact_display_path(path: &Path) -> String {
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

pub(super) fn resolve_quantitative_audit_artifact_path(
	manifest_path: &Path,
	artifact_path: &str,
) -> PathBuf {
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
