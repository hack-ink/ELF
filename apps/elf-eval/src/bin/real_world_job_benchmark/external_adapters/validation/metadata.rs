use std::path::Path;

use color_eyre::{Result, eyre};

use crate::ExternalAdapterReport;

pub(in crate::external_adapters::validation) fn validate_adapter_execution_metadata(
	path: &Path,
	adapter: &ExternalAdapterReport,
) -> Result<()> {
	let Some(metadata) = &adapter.execution_metadata else {
		return Ok(());
	};

	if metadata.setup_path.trim().is_empty()
		|| metadata.runtime_boundary.trim().is_empty()
		|| metadata.resource_expectation.trim().is_empty()
		|| metadata.retry_guidance.iter().any(|guidance| guidance.trim().is_empty())
		|| metadata.sources.is_empty()
	{
		return Err(eyre::eyre!(
			"{} adapter {} has incomplete execution metadata.",
			path.display(),
			adapter.adapter_id
		));
	}

	for source in &metadata.sources {
		if source.label.trim().is_empty()
			|| source.url.trim().is_empty()
			|| source.evidence.trim().is_empty()
		{
			return Err(eyre::eyre!(
				"{} adapter {} has incomplete source metadata.",
				path.display(),
				adapter.adapter_id
			));
		}
	}

	Ok(())
}
