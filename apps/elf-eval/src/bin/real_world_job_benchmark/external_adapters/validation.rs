use std::{collections::BTreeSet, path::Path};

use color_eyre::{Result, eyre};

use crate::{
	AdapterCoverageStatus, AdapterScenarioJudgment, EXTERNAL_ADAPTER_MANIFEST_SCHEMA,
	ElfScenarioPosition, ExternalAdapterManifest, ExternalAdapterReport, ExternalDockerIsolation,
	SUITES, ScenarioComparisonOutcome, external_adapters::outcome, formatting,
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

	validate_adapter_execution(path, adapter)?;
	validate_adapter_capabilities(path, adapter)?;
	validate_adapter_suites(path, adapter)?;
	validate_adapter_scenarios(path, adapter)?;
	validate_adapter_evidence(path, adapter)?;
	validate_adapter_execution_metadata(path, adapter)?;

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

fn validate_adapter_execution(path: &Path, adapter: &ExternalAdapterReport) -> Result<()> {
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

fn validate_adapter_capabilities(path: &Path, adapter: &ExternalAdapterReport) -> Result<()> {
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

fn validate_adapter_suites(path: &Path, adapter: &ExternalAdapterReport) -> Result<()> {
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

fn validate_adapter_scenarios(path: &Path, adapter: &ExternalAdapterReport) -> Result<()> {
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

fn validate_adapter_evidence(path: &Path, adapter: &ExternalAdapterReport) -> Result<()> {
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

fn validate_adapter_execution_metadata(path: &Path, adapter: &ExternalAdapterReport) -> Result<()> {
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
