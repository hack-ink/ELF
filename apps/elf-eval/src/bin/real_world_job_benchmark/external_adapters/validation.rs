mod basics;
mod metadata;
mod scenarios;

use std::{collections::BTreeSet, path::Path};

use color_eyre::{Result, eyre};

use crate::{
	EXTERNAL_ADAPTER_MANIFEST_SCHEMA, ExternalAdapterManifest, ExternalAdapterReport,
	ExternalDockerIsolation,
};

pub(super) fn validate_external_adapter_manifest(
	manifest: &ExternalAdapterManifest,
	path: &Path,
) -> Result<()> {
	if manifest.schema != EXTERNAL_ADAPTER_MANIFEST_SCHEMA {
		return Err(eyre::eyre!(
			"{} has schema {}, expected {EXTERNAL_ADAPTER_MANIFEST_SCHEMA}.",
			path.display(),
			manifest.schema
		));
	}
	if manifest.manifest_id.trim().is_empty() {
		return Err(eyre::eyre!("{} has an empty manifest_id.", path.display()));
	}

	validate_external_docker_isolation(path, &manifest.docker_isolation)?;

	validate_external_adapters(path, &manifest.adapters)
}

fn validate_external_docker_isolation(path: &Path, docker: &ExternalDockerIsolation) -> Result<()> {
	if docker.compose_file.trim().is_empty()
		|| docker.runner.trim().is_empty()
		|| docker.artifact_dir.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has incomplete docker_isolation metadata.", path.display()));
	}
	if !docker.default {
		return Err(eyre::eyre!(
			"{} external adapter manifest must default to Docker isolation.",
			path.display()
		));
	}
	if docker.host_global_installs_required {
		return Err(eyre::eyre!(
			"{} external adapter manifest must not require host-global installs by default.",
			path.display()
		));
	}

	Ok(())
}

fn validate_external_adapters(path: &Path, adapters: &[ExternalAdapterReport]) -> Result<()> {
	if adapters.is_empty() {
		return Err(eyre::eyre!("{} declares no external adapters.", path.display()));
	}

	let mut seen = BTreeSet::new();

	for adapter in adapters {
		validate_external_adapter(path, adapter)?;

		if !seen.insert(adapter.adapter_id.as_str()) {
			return Err(eyre::eyre!(
				"{} declares duplicate adapter_id {}.",
				path.display(),
				adapter.adapter_id
			));
		}
	}

	Ok(())
}

fn validate_external_adapter(path: &Path, adapter: &ExternalAdapterReport) -> Result<()> {
	if adapter.adapter_id.trim().is_empty()
		|| adapter.project.trim().is_empty()
		|| adapter.adapter_kind.trim().is_empty()
		|| adapter.evidence_class.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete external adapter.", path.display()));
	}
	if !matches!(
		adapter.evidence_class.as_str(),
		"fixture_backed" | "live_baseline_only" | "live_real_world" | "research_gate"
	) {
		return Err(eyre::eyre!(
			"{} adapter {} has unsupported evidence_class {}.",
			path.display(),
			adapter.adapter_id,
			adapter.evidence_class
		));
	}
	if adapter.docker_default && adapter.host_global_installs_required {
		return Err(eyre::eyre!(
			"{} adapter {} is Docker-default but requires host-global installs.",
			path.display(),
			adapter.adapter_id
		));
	}

	basics::validate_adapter_execution(path, adapter)?;
	basics::validate_adapter_capabilities(path, adapter)?;
	basics::validate_adapter_suites(path, adapter)?;
	scenarios::validate_adapter_scenarios(path, adapter)?;
	basics::validate_adapter_evidence(path, adapter)?;
	metadata::validate_adapter_execution_metadata(path, adapter)?;

	if let Some(follow_up) = &adapter.follow_up
		&& (follow_up.title.trim().is_empty() || follow_up.reason.trim().is_empty())
	{
		return Err(eyre::eyre!(
			"{} adapter {} has an incomplete follow_up.",
			path.display(),
			adapter.adapter_id
		));
	}

	Ok(())
}
