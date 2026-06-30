use std::path::Path;

use color_eyre::{Result, eyre};

use crate::{ExternalAdapterReport, SUITES};

pub(in crate::external_adapters::validation) fn validate_adapter_execution(
	path: &Path,
	adapter: &ExternalAdapterReport,
) -> Result<()> {
	for evidence in [&adapter.setup, &adapter.run, &adapter.result] {
		if evidence.evidence.trim().is_empty()
			|| evidence.command.as_deref().is_some_and(str::is_empty)
			|| evidence.artifact.as_deref().is_some_and(str::is_empty)
		{
			return Err(eyre::eyre!(
				"{} adapter {} has incomplete setup/run/result evidence.",
				path.display(),
				adapter.adapter_id
			));
		}
	}

	Ok(())
}

pub(in crate::external_adapters::validation) fn validate_adapter_capabilities(
	path: &Path,
	adapter: &ExternalAdapterReport,
) -> Result<()> {
	for capability in &adapter.capabilities {
		if capability.capability.trim().is_empty() || capability.evidence.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} adapter {} has incomplete capability coverage.",
				path.display(),
				adapter.adapter_id
			));
		}
	}

	Ok(())
}

pub(in crate::external_adapters::validation) fn validate_adapter_suites(
	path: &Path,
	adapter: &ExternalAdapterReport,
) -> Result<()> {
	for suite in &adapter.suites {
		if !SUITES.contains(&suite.suite_id.as_str()) {
			return Err(eyre::eyre!(
				"{} adapter {} references unknown suite {}.",
				path.display(),
				adapter.adapter_id,
				suite.suite_id
			));
		}
		if suite.evidence.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} adapter {} has suite {} without evidence.",
				path.display(),
				adapter.adapter_id,
				suite.suite_id
			));
		}
	}

	Ok(())
}

pub(in crate::external_adapters::validation) fn validate_adapter_evidence(
	path: &Path,
	adapter: &ExternalAdapterReport,
) -> Result<()> {
	for evidence in &adapter.evidence {
		if evidence.kind.trim().is_empty() || evidence.reference.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} adapter {} has incomplete evidence pointers.",
				path.display(),
				adapter.adapter_id
			));
		}
	}

	Ok(())
}
