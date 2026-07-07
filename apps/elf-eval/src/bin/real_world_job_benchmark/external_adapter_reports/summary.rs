use crate::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct ExternalAdapterSummary {
	pub(crate) adapter_count: usize,
	pub(crate) external_project_count: usize,
	pub(crate) docker_default_count: usize,
	pub(crate) host_global_install_required_count: usize,
	pub(crate) fixture_backed_count: usize,
	pub(crate) live_baseline_only_count: usize,
	pub(crate) live_real_world_count: usize,
	#[serde(default)]
	pub(crate) research_gate_count: usize,
	pub(crate) overall_status_counts: AdapterStatusCounts,
	pub(crate) capability_status_counts: AdapterStatusCounts,
	pub(crate) suite_status_counts: AdapterStatusCounts,
	#[serde(default)]
	pub(crate) scenario_status_counts: AdapterStatusCounts,
	#[serde(default)]
	pub(crate) scenario_position_counts: ScenarioPositionCounts,
	#[serde(default)]
	pub(crate) scenario_outcome_counts: ScenarioOutcomeCounts,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct AdapterStatusCounts {
	pub(crate) real: usize,
	pub(crate) mocked: usize,
	pub(crate) unsupported: usize,
	pub(crate) blocked: usize,
	pub(crate) incomplete: usize,
	pub(crate) wrong_result: usize,
	pub(crate) lifecycle_fail: usize,
	pub(crate) pass: usize,
	pub(crate) not_encoded: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct ScenarioPositionCounts {
	pub(crate) wins: usize,
	pub(crate) ties: usize,
	pub(crate) loses: usize,
	pub(crate) untested: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct ScenarioOutcomeCounts {
	pub(crate) win: usize,
	pub(crate) tie: usize,
	pub(crate) loss: usize,
	pub(crate) not_tested: usize,
	pub(crate) blocked: usize,
	pub(crate) non_goal: usize,
}
