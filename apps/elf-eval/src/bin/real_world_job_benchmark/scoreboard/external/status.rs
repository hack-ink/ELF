use crate::scoreboard::{AdapterCoverageStatus, ExternalAdapterReport, common};

pub(super) fn strongest_scoreboard_evidence_class(adapters: &[&ExternalAdapterReport]) -> String {
	for evidence_class in ["live_real_world", "live_baseline", "fixture_backed", "research_gate"] {
		if adapters.iter().any(|adapter| {
			common::scoreboard_evidence_class(adapter.evidence_class.as_str()) == evidence_class
		}) {
			return evidence_class.to_string();
		}
	}

	"research_gate".to_string()
}

pub(super) fn external_project_result_state(adapters: &[&ExternalAdapterReport]) -> String {
	for status in [
		AdapterCoverageStatus::WrongResult,
		AdapterCoverageStatus::Blocked,
		AdapterCoverageStatus::Incomplete,
		AdapterCoverageStatus::LifecycleFail,
		AdapterCoverageStatus::NotEncoded,
		AdapterCoverageStatus::Unsupported,
	] {
		if adapters.iter().any(|adapter| adapter_has_status(adapter, status)) {
			return adapter_status_to_scoreboard_state(status).to_string();
		}
	}

	"not_comparable".to_string()
}

pub(super) fn adapter_typed_non_pass_count(adapter: &ExternalAdapterReport) -> usize {
	let direct_statuses =
		[adapter.overall_status, adapter.setup.status, adapter.run.status, adapter.result.status];
	let direct = direct_statuses
		.into_iter()
		.filter(|status| adapter_status_is_typed_non_pass(*status))
		.count();
	let capability = adapter
		.capabilities
		.iter()
		.filter(|capability| adapter_status_is_typed_non_pass(capability.status))
		.count();
	let suites = adapter
		.suites
		.iter()
		.filter(|suite| adapter_status_is_typed_non_pass(suite.status))
		.count();
	let scenarios = adapter
		.scenarios
		.iter()
		.filter(|scenario| adapter_status_is_typed_non_pass(scenario.status))
		.count();

	direct + capability + suites + scenarios
}

pub(super) fn adapter_status_is_typed_non_pass(status: AdapterCoverageStatus) -> bool {
	matches!(
		status,
		AdapterCoverageStatus::Unsupported
			| AdapterCoverageStatus::Blocked
			| AdapterCoverageStatus::Incomplete
			| AdapterCoverageStatus::WrongResult
			| AdapterCoverageStatus::LifecycleFail
			| AdapterCoverageStatus::NotEncoded
	)
}

fn adapter_has_status(adapter: &ExternalAdapterReport, status: AdapterCoverageStatus) -> bool {
	adapter.overall_status == status
		|| adapter.setup.status == status
		|| adapter.run.status == status
		|| adapter.result.status == status
		|| adapter.capabilities.iter().any(|capability| capability.status == status)
		|| adapter.suites.iter().any(|suite| suite.status == status)
		|| adapter.scenarios.iter().any(|scenario| scenario.status == status)
}

fn adapter_status_to_scoreboard_state(status: AdapterCoverageStatus) -> &'static str {
	match status {
		AdapterCoverageStatus::WrongResult | AdapterCoverageStatus::LifecycleFail => "wrong_result",
		AdapterCoverageStatus::Blocked => "blocked",
		AdapterCoverageStatus::Incomplete => "incomplete",
		AdapterCoverageStatus::NotEncoded | AdapterCoverageStatus::Unsupported => "not_encoded",
		AdapterCoverageStatus::Real
		| AdapterCoverageStatus::Mocked
		| AdapterCoverageStatus::Pass => "not_comparable",
	}
}
