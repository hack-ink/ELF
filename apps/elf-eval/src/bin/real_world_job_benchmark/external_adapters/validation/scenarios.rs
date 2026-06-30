use std::path::Path;

use color_eyre::{Result, eyre};

use crate::{
	AdapterCoverageStatus, AdapterScenarioJudgment, ElfScenarioPosition, ExternalAdapterReport,
	SUITES, ScenarioComparisonOutcome, external_adapters::outcome, formatting,
};

pub(in crate::external_adapters::validation) fn validate_adapter_scenarios(
	path: &Path,
	adapter: &ExternalAdapterReport,
) -> Result<()> {
	for scenario in &adapter.scenarios {
		if scenario.scenario_id.trim().is_empty()
			|| scenario.evidence.trim().is_empty()
			|| scenario.command.as_deref().is_some_and(str::is_empty)
			|| scenario.artifact.as_deref().is_some_and(str::is_empty)
		{
			return Err(eyre::eyre!(
				"{} adapter {} has incomplete scenario judgment.",
				path.display(),
				adapter.adapter_id
			));
		}

		if let Some(suite_id) = &scenario.suite_id
			&& !SUITES.contains(&suite_id.as_str())
		{
			return Err(eyre::eyre!(
				"{} adapter {} scenario {} references unknown suite {}.",
				path.display(),
				adapter.adapter_id,
				scenario.scenario_id,
				suite_id
			));
		}

		let outcome = outcome::scenario_comparison_outcome(scenario);

		if blocked_status_missing_blocked_outcome(scenario.status, scenario.comparison_outcome) {
			return Err(eyre::eyre!(
				"{} adapter {} scenario {} uses blocked status without blocked comparison outcome.",
				path.display(),
				adapter.adapter_id,
				scenario.scenario_id
			));
		}
		if unmeasured_status_has_measured_outcome(scenario.status, outcome) {
			return Err(eyre::eyre!(
				"{} adapter {} scenario {} uses {} status with {} outcome.",
				path.display(),
				adapter.adapter_id,
				scenario.scenario_id,
				formatting::adapter_status_str(scenario.status),
				formatting::scenario_comparison_outcome_str(outcome)
			));
		}
		if unmeasured_status_has_measured_position(scenario.status, scenario.elf_position) {
			return Err(eyre::eyre!(
				"{} adapter {} scenario {} uses {} status with {} position.",
				path.display(),
				adapter.adapter_id,
				scenario.scenario_id,
				formatting::adapter_status_str(scenario.status),
				formatting::scenario_position_str(scenario.elf_position)
			));
		}
		if explicit_outcome_conflicts_with_position(scenario) {
			return Err(eyre::eyre!(
				"{} adapter {} scenario {} uses {} position with {} outcome.",
				path.display(),
				adapter.adapter_id,
				scenario.scenario_id,
				formatting::scenario_position_str(scenario.elf_position),
				formatting::scenario_comparison_outcome_str(outcome)
			));
		}
	}

	Ok(())
}

fn blocked_status_missing_blocked_outcome(
	status: AdapterCoverageStatus,
	outcome: Option<ScenarioComparisonOutcome>,
) -> bool {
	status == AdapterCoverageStatus::Blocked && outcome != Some(ScenarioComparisonOutcome::Blocked)
}

fn unmeasured_status_has_measured_outcome(
	status: AdapterCoverageStatus,
	outcome: ScenarioComparisonOutcome,
) -> bool {
	matches!(
		status,
		AdapterCoverageStatus::Blocked
			| AdapterCoverageStatus::Incomplete
			| AdapterCoverageStatus::NotEncoded
			| AdapterCoverageStatus::Unsupported
	) && matches!(
		outcome,
		ScenarioComparisonOutcome::Win
			| ScenarioComparisonOutcome::Tie
			| ScenarioComparisonOutcome::Loss
	)
}

fn unmeasured_status_has_measured_position(
	status: AdapterCoverageStatus,
	position: ElfScenarioPosition,
) -> bool {
	matches!(
		status,
		AdapterCoverageStatus::Blocked
			| AdapterCoverageStatus::Incomplete
			| AdapterCoverageStatus::NotEncoded
			| AdapterCoverageStatus::Unsupported
	) && matches!(
		position,
		ElfScenarioPosition::Wins | ElfScenarioPosition::Ties | ElfScenarioPosition::Loses
	)
}

fn explicit_outcome_conflicts_with_position(scenario: &AdapterScenarioJudgment) -> bool {
	let Some(outcome) = scenario.comparison_outcome else {
		return false;
	};

	!outcome::position_supports_outcome(scenario.elf_position, outcome)
}
