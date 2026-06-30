use crate::markdown::{AdapterStatusCounts, ScenarioOutcomeCounts, ScenarioPositionCounts};

pub(in crate::markdown::adapters) fn adapter_status_counts_display(
	counts: &AdapterStatusCounts,
) -> String {
	[
		("real", counts.real),
		("mocked", counts.mocked),
		("unsupported", counts.unsupported),
		("blocked", counts.blocked),
		("incomplete", counts.incomplete),
		("wrong_result", counts.wrong_result),
		("lifecycle_fail", counts.lifecycle_fail),
		("pass", counts.pass),
		("not_encoded", counts.not_encoded),
	]
	.into_iter()
	.filter(|(_, count)| *count > 0)
	.map(|(status, count)| format!("{status}={count}"))
	.collect::<Vec<_>>()
	.join(", ")
}

pub(in crate::markdown::adapters) fn scenario_position_counts_display(
	counts: &ScenarioPositionCounts,
) -> String {
	[
		("wins", counts.wins),
		("ties", counts.ties),
		("loses", counts.loses),
		("untested", counts.untested),
	]
	.into_iter()
	.filter(|(_, count)| *count > 0)
	.map(|(position, count)| format!("{position}={count}"))
	.collect::<Vec<_>>()
	.join(", ")
}

pub(in crate::markdown::adapters) fn scenario_outcome_counts_display(
	counts: &ScenarioOutcomeCounts,
) -> String {
	[
		("win", counts.win),
		("tie", counts.tie),
		("loss", counts.loss),
		("not_tested", counts.not_tested),
		("blocked", counts.blocked),
		("non_goal", counts.non_goal),
	]
	.into_iter()
	.filter(|(_, count)| *count > 0)
	.map(|(outcome, count)| format!("{outcome}={count}"))
	.collect::<Vec<_>>()
	.join(", ")
}
