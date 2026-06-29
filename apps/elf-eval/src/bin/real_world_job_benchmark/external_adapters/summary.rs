use std::collections::BTreeSet;

use crate::{
	AdapterCoverageStatus, AdapterStatusCounts, ElfScenarioPosition, ExternalAdapterReport,
	ExternalAdapterSummary, ScenarioComparisonOutcome, ScenarioOutcomeCounts,
	ScenarioPositionCounts, external_adapters::outcome,
};

pub(super) fn external_adapter_summary(
	adapters: &[ExternalAdapterReport],
) -> ExternalAdapterSummary {
	let external_projects = adapters
		.iter()
		.filter_map(|adapter| (adapter.project != "ELF").then_some(adapter.project.as_str()))
		.collect::<BTreeSet<_>>();
	let mut summary = ExternalAdapterSummary {
		adapter_count: adapters.len(),
		external_project_count: external_projects.len(),
		..ExternalAdapterSummary::default()
	};

	for adapter in adapters {
		accumulate_adapter_summary(&mut summary, adapter);
	}

	summary
}

fn accumulate_adapter_summary(
	summary: &mut ExternalAdapterSummary,
	adapter: &ExternalAdapterReport,
) {
	summary.docker_default_count += usize::from(adapter.docker_default);
	summary.host_global_install_required_count +=
		usize::from(adapter.host_global_installs_required);
	summary.fixture_backed_count += usize::from(adapter.evidence_class == "fixture_backed");
	summary.live_baseline_only_count += usize::from(adapter.evidence_class == "live_baseline_only");
	summary.live_real_world_count += usize::from(adapter.evidence_class == "live_real_world");
	summary.research_gate_count += usize::from(adapter.evidence_class == "research_gate");

	increment_adapter_status_count(&mut summary.overall_status_counts, adapter.overall_status);

	for capability in &adapter.capabilities {
		increment_adapter_status_count(&mut summary.capability_status_counts, capability.status);
	}
	for suite in &adapter.suites {
		increment_adapter_status_count(&mut summary.suite_status_counts, suite.status);
	}
	for scenario in &adapter.scenarios {
		increment_adapter_status_count(&mut summary.scenario_status_counts, scenario.status);
		increment_scenario_position_count(
			&mut summary.scenario_position_counts,
			scenario.elf_position,
		);
		increment_scenario_outcome_count(
			&mut summary.scenario_outcome_counts,
			outcome::scenario_comparison_outcome(scenario),
		);
	}
}

fn increment_adapter_status_count(counts: &mut AdapterStatusCounts, status: AdapterCoverageStatus) {
	match status {
		AdapterCoverageStatus::Real => counts.real += 1,
		AdapterCoverageStatus::Mocked => counts.mocked += 1,
		AdapterCoverageStatus::Unsupported => counts.unsupported += 1,
		AdapterCoverageStatus::Blocked => counts.blocked += 1,
		AdapterCoverageStatus::Incomplete => counts.incomplete += 1,
		AdapterCoverageStatus::WrongResult => counts.wrong_result += 1,
		AdapterCoverageStatus::LifecycleFail => counts.lifecycle_fail += 1,
		AdapterCoverageStatus::Pass => counts.pass += 1,
		AdapterCoverageStatus::NotEncoded => counts.not_encoded += 1,
	}
}

fn increment_scenario_position_count(
	counts: &mut ScenarioPositionCounts,
	position: ElfScenarioPosition,
) {
	match position {
		ElfScenarioPosition::Wins => counts.wins += 1,
		ElfScenarioPosition::Ties => counts.ties += 1,
		ElfScenarioPosition::Loses => counts.loses += 1,
		ElfScenarioPosition::Untested => counts.untested += 1,
	}
}

fn increment_scenario_outcome_count(
	counts: &mut ScenarioOutcomeCounts,
	outcome: ScenarioComparisonOutcome,
) {
	match outcome {
		ScenarioComparisonOutcome::Win => counts.win += 1,
		ScenarioComparisonOutcome::Tie => counts.tie += 1,
		ScenarioComparisonOutcome::Loss => counts.loss += 1,
		ScenarioComparisonOutcome::NotTested => counts.not_tested += 1,
		ScenarioComparisonOutcome::Blocked => counts.blocked += 1,
		ScenarioComparisonOutcome::NonGoal => counts.non_goal += 1,
	}
}
