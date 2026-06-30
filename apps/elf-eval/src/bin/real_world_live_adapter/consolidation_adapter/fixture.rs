use crate::{LiveConsolidationFixture, LoadedJob, Result, eyre, serde_json};

pub(in crate::consolidation_adapter) fn live_consolidation_fixture(
	loaded: &LoadedJob,
) -> Result<LiveConsolidationFixture> {
	let value =
		loaded.value.pointer("/corpus/adapter_response/consolidation").cloned().ok_or_else(
			|| {
				eyre::eyre!(
					"{} does not contain adapter_response.consolidation.",
					loaded.path.display()
				)
			},
		)?;

	serde_json::from_value(value).map_err(|err| {
		eyre::eyre!("Failed to parse consolidation fixture {}: {err}", loaded.path.display())
	})
}
