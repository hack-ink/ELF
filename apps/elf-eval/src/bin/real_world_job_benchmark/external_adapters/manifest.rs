use std::path::{Path, PathBuf};

use crate::{
	EXTERNAL_ADAPTER_REPORT_SCHEMA, ExternalAdapterSection, ExternalAdapterSummary,
	ExternalDockerIsolation,
};

pub(super) fn empty_external_adapter_section(reason: &str) -> ExternalAdapterSection {
	ExternalAdapterSection {
		schema: EXTERNAL_ADAPTER_REPORT_SCHEMA.to_string(),
		manifest_id: reason.to_string(),
		docker_isolation: ExternalDockerIsolation::default(),
		summary: ExternalAdapterSummary::default(),
		adapters: Vec::new(),
	}
}

pub(super) fn resolve_external_adapter_manifest_path(path: &Path) -> PathBuf {
	if path.exists() || path.is_absolute() {
		return path.to_path_buf();
	}

	let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
	let Some(workspace_root) = manifest_dir.parent().and_then(Path::parent) else {
		return path.to_path_buf();
	};
	let workspace_candidate = workspace_root.join(path);

	if workspace_candidate.exists() { workspace_candidate } else { path.to_path_buf() }
}
