use super::super::{AdapterScenarioJudgment, ElfScenarioPosition, ScenarioComparisonOutcome};

pub(in super::super) fn scenario_comparison_outcome(
	scenario: &AdapterScenarioJudgment,
) -> ScenarioComparisonOutcome {
	scenario.comparison_outcome.unwrap_or(match scenario.elf_position {
		ElfScenarioPosition::Wins => ScenarioComparisonOutcome::Win,
		ElfScenarioPosition::Ties => ScenarioComparisonOutcome::Tie,
		ElfScenarioPosition::Loses => ScenarioComparisonOutcome::Loss,
		ElfScenarioPosition::Untested => ScenarioComparisonOutcome::NotTested,
	})
}

pub(super) fn position_supports_outcome(
	position: ElfScenarioPosition,
	outcome: ScenarioComparisonOutcome,
) -> bool {
	matches!(
		(position, outcome),
		(ElfScenarioPosition::Wins, ScenarioComparisonOutcome::Win)
			| (ElfScenarioPosition::Ties, ScenarioComparisonOutcome::Tie)
			| (ElfScenarioPosition::Loses, ScenarioComparisonOutcome::Loss)
			| (ElfScenarioPosition::Untested, ScenarioComparisonOutcome::NotTested)
			| (ElfScenarioPosition::Untested, ScenarioComparisonOutcome::Blocked)
			| (ElfScenarioPosition::Untested, ScenarioComparisonOutcome::NonGoal)
	)
}
