use super::*;

pub(super) fn status_str(status: TypedStatus) -> &'static str {
	match status {
		TypedStatus::Pass => "pass",
		TypedStatus::WrongResult => "wrong_result",
		TypedStatus::LifecycleFail => "lifecycle_fail",
		TypedStatus::Incomplete => "incomplete",
		TypedStatus::Blocked => "blocked",
		TypedStatus::NotEncoded => "not_encoded",
		TypedStatus::UnsupportedClaim => "unsupported_claim",
	}
}

pub(super) fn adapter_status_str(status: AdapterCoverageStatus) -> &'static str {
	match status {
		AdapterCoverageStatus::Real => "real",
		AdapterCoverageStatus::Mocked => "mocked",
		AdapterCoverageStatus::Unsupported => "unsupported",
		AdapterCoverageStatus::Blocked => "blocked",
		AdapterCoverageStatus::Incomplete => "incomplete",
		AdapterCoverageStatus::WrongResult => "wrong_result",
		AdapterCoverageStatus::LifecycleFail => "lifecycle_fail",
		AdapterCoverageStatus::Pass => "pass",
		AdapterCoverageStatus::NotEncoded => "not_encoded",
	}
}

pub(super) fn scenario_comparison_outcome_str(outcome: ScenarioComparisonOutcome) -> &'static str {
	match outcome {
		ScenarioComparisonOutcome::Win => "win",
		ScenarioComparisonOutcome::Tie => "tie",
		ScenarioComparisonOutcome::Loss => "loss",
		ScenarioComparisonOutcome::NotTested => "not_tested",
		ScenarioComparisonOutcome::Blocked => "blocked",
		ScenarioComparisonOutcome::NonGoal => "non_goal",
	}
}

pub(super) fn scenario_position_str(position: ElfScenarioPosition) -> &'static str {
	match position {
		ElfScenarioPosition::Wins => "wins",
		ElfScenarioPosition::Ties => "ties",
		ElfScenarioPosition::Loses => "loses",
		ElfScenarioPosition::Untested => "untested",
	}
}

pub(super) fn trace_failure_stage(trace: Option<&TraceExplainability>) -> Option<&str> {
	trace.and_then(|trace| trace.failure_stage.as_deref())
}

pub(super) fn bounded_text(value: &str, max_chars: usize) -> String {
	let mut chars = value.chars();
	let text = chars.by_ref().take(max_chars).collect::<String>();

	if chars.next().is_some() { format!("{text}...") } else { text }
}

pub(super) fn round3(value: f64) -> f64 {
	(value * 1_000.0).round() / 1_000.0
}
