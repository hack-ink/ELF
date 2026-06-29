use super::*;

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct AdapterReport {
	pub(super) adapter_id: String,
	pub(super) name: String,
	pub(super) behavior: String,
	pub(super) storage: TypedStatus,
	pub(super) runtime: TypedStatus,
	pub(super) notes: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ExternalAdapterManifest {
	pub(super) schema: String,
	pub(super) manifest_id: String,
	pub(super) docker_isolation: ExternalDockerIsolation,
	#[serde(default)]
	pub(super) adapters: Vec<ExternalAdapterReport>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ExternalAdapterSection {
	pub(super) schema: String,
	pub(super) manifest_id: String,
	pub(super) docker_isolation: ExternalDockerIsolation,
	pub(super) summary: ExternalAdapterSummary,
	#[serde(default)]
	pub(super) adapters: Vec<ExternalAdapterReport>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ExternalDockerIsolation {
	pub(super) default: bool,
	pub(super) compose_file: String,
	pub(super) runner: String,
	pub(super) artifact_dir: String,
	pub(super) host_global_installs_required: bool,
	#[serde(default)]
	pub(super) notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ExternalAdapterReport {
	pub(super) adapter_id: String,
	pub(super) project: String,
	pub(super) adapter_kind: String,
	pub(super) evidence_class: String,
	pub(super) docker_default: bool,
	pub(super) host_global_installs_required: bool,
	pub(super) overall_status: AdapterCoverageStatus,
	pub(super) setup: AdapterExecutionEvidence,
	pub(super) run: AdapterExecutionEvidence,
	pub(super) result: AdapterExecutionEvidence,
	#[serde(default)]
	pub(super) capabilities: Vec<AdapterCapabilityCoverage>,
	#[serde(default)]
	pub(super) suites: Vec<AdapterSuiteCoverage>,
	#[serde(default)]
	pub(super) scenarios: Vec<AdapterScenarioJudgment>,
	#[serde(default)]
	pub(super) evidence: Vec<AdapterEvidencePointer>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) execution_metadata: Option<AdapterExecutionMetadata>,
	#[serde(default)]
	pub(super) notes: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) follow_up: Option<FollowUpInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct AdapterExecutionEvidence {
	pub(super) status: AdapterCoverageStatus,
	pub(super) evidence: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) command: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) artifact: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct AdapterCapabilityCoverage {
	pub(super) capability: String,
	pub(super) status: AdapterCoverageStatus,
	pub(super) evidence: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct AdapterSuiteCoverage {
	pub(super) suite_id: String,
	pub(super) status: AdapterCoverageStatus,
	pub(super) evidence: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct AdapterScenarioJudgment {
	pub(super) scenario_id: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) suite_id: Option<String>,
	pub(super) status: AdapterCoverageStatus,
	pub(super) elf_position: ElfScenarioPosition,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) comparison_outcome: Option<ScenarioComparisonOutcome>,
	pub(super) evidence: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) command: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) artifact: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct AdapterEvidencePointer {
	pub(super) kind: String,
	#[serde(rename = "ref")]
	pub(super) reference: String,
	pub(super) status: AdapterCoverageStatus,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct AdapterExecutionMetadata {
	#[serde(default)]
	pub(super) sources: Vec<AdapterSource>,
	pub(super) setup_path: String,
	pub(super) runtime_boundary: String,
	pub(super) resource_expectation: String,
	#[serde(default)]
	pub(super) retry_guidance: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) research_depth: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct AdapterSource {
	pub(super) label: String,
	pub(super) url: String,
	pub(super) evidence: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ExternalAdapterSummary {
	pub(super) adapter_count: usize,
	pub(super) external_project_count: usize,
	pub(super) docker_default_count: usize,
	pub(super) host_global_install_required_count: usize,
	pub(super) fixture_backed_count: usize,
	pub(super) live_baseline_only_count: usize,
	pub(super) live_real_world_count: usize,
	#[serde(default)]
	pub(super) research_gate_count: usize,
	pub(super) overall_status_counts: AdapterStatusCounts,
	pub(super) capability_status_counts: AdapterStatusCounts,
	pub(super) suite_status_counts: AdapterStatusCounts,
	#[serde(default)]
	pub(super) scenario_status_counts: AdapterStatusCounts,
	#[serde(default)]
	pub(super) scenario_position_counts: ScenarioPositionCounts,
	#[serde(default)]
	pub(super) scenario_outcome_counts: ScenarioOutcomeCounts,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct AdapterStatusCounts {
	pub(super) real: usize,
	pub(super) mocked: usize,
	pub(super) unsupported: usize,
	pub(super) blocked: usize,
	pub(super) incomplete: usize,
	pub(super) wrong_result: usize,
	pub(super) lifecycle_fail: usize,
	pub(super) pass: usize,
	pub(super) not_encoded: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ScenarioPositionCounts {
	pub(super) wins: usize,
	pub(super) ties: usize,
	pub(super) loses: usize,
	pub(super) untested: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct ScenarioOutcomeCounts {
	pub(super) win: usize,
	pub(super) tie: usize,
	pub(super) loss: usize,
	pub(super) not_tested: usize,
	pub(super) blocked: usize,
	pub(super) non_goal: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct CaptureIntegrationReport {
	#[serde(default)]
	pub(super) real: Vec<String>,
	#[serde(default)]
	pub(super) fixture_backed: Vec<String>,
	#[serde(default)]
	pub(super) mocked: Vec<String>,
	#[serde(default)]
	pub(super) blocked: Vec<String>,
	#[serde(default)]
	pub(super) not_encoded: Vec<String>,
	#[serde(default)]
	pub(super) notes: Vec<String>,
}
