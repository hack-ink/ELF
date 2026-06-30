mod manifest;
mod outcome;
mod summary;
mod validation;

pub(super) use outcome::scenario_comparison_outcome;

use std::fs;

use crate::{
	EXTERNAL_ADAPTER_REPORT_SCHEMA, ExternalAdapterManifest, ExternalAdapterSection, Path, Result,
	eyre,
};

pub(super) fn external_adapter_section(
	manifest_path: &Path,
	skip_manifest: bool,
) -> Result<ExternalAdapterSection> {
	if skip_manifest {
		return Ok(manifest::empty_external_adapter_section("skipped"));
	}

	let manifest_path = manifest::resolve_external_adapter_manifest_path(manifest_path);

	if !manifest_path.exists() {
		return Ok(manifest::empty_external_adapter_section("missing"));
	}

	let raw = fs::read_to_string(&manifest_path)?;
	let manifest = serde_json::from_str::<ExternalAdapterManifest>(&raw).map_err(|err| {
		eyre::eyre!("Failed to parse external adapter manifest {}: {err}", manifest_path.display())
	})?;

	validation::validate_external_adapter_manifest(&manifest, &manifest_path)?;

	let summary = summary::external_adapter_summary(&manifest.adapters);

	Ok(ExternalAdapterSection {
		schema: EXTERNAL_ADAPTER_REPORT_SCHEMA.to_string(),
		manifest_id: manifest.manifest_id,
		docker_isolation: manifest.docker_isolation,
		summary,
		adapters: manifest.adapters,
	})
}
