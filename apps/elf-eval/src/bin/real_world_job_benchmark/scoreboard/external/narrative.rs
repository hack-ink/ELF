use crate::scoreboard::{
	self, AdapterCoverageStatus, BTreeSet, ExternalAdapterReport, ScenarioComparisonOutcome,
	external::status,
};

pub(super) fn external_project_strengths(adapters: &[&ExternalAdapterReport]) -> Vec<String> {
	let mut strengths = BTreeSet::new();

	for adapter in adapters {
		for capability in &adapter.capabilities {
			if matches!(
				capability.status,
				AdapterCoverageStatus::Pass | AdapterCoverageStatus::Real
			) {
				strengths.insert(format!(
					"{} capability is {}.",
					capability.capability,
					scoreboard::adapter_status_str(capability.status)
				));
			}
		}
		for scenario in &adapter.scenarios {
			if scoreboard::scenario_comparison_outcome(scenario) == ScenarioComparisonOutcome::Loss
			{
				strengths.insert(format!(
					"Scenario {} is recorded as a competitor strength.",
					scenario.scenario_id
				));
			}
		}
	}

	strengths.into_iter().take(6).collect()
}

pub(super) fn external_project_weaknesses(adapters: &[&ExternalAdapterReport]) -> Vec<String> {
	let mut weaknesses = BTreeSet::new();

	for adapter in adapters {
		if adapter.overall_status != AdapterCoverageStatus::Pass {
			weaknesses.insert(format!(
				"Adapter {} overall status is {}.",
				adapter.adapter_id,
				scoreboard::adapter_status_str(adapter.overall_status)
			));
		}

		for suite in &adapter.suites {
			if status::adapter_status_is_typed_non_pass(suite.status) {
				weaknesses.insert(format!(
					"Suite {} is {}.",
					suite.suite_id,
					scoreboard::adapter_status_str(suite.status)
				));
			}
		}
	}

	weaknesses.into_iter().take(8).collect()
}

pub(super) fn external_project_source_provenance(
	adapters: &[&ExternalAdapterReport],
) -> Vec<String> {
	let mut provenance = BTreeSet::new();

	for adapter in adapters {
		for evidence in &adapter.evidence {
			provenance.insert(evidence.reference.clone());
		}
		for artifact in [&adapter.setup.artifact, &adapter.run.artifact, &adapter.result.artifact]
			.into_iter()
			.flatten()
		{
			provenance.insert(artifact.clone());
		}
	}

	provenance.into_iter().take(12).collect()
}
