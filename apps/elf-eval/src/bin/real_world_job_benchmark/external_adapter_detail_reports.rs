use crate::{
	AdapterCoverageStatus, Deserialize, ElfScenarioPosition, FollowUpInput,
	ScenarioComparisonOutcome, Serialize,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ExternalAdapterReport {
	pub(crate) adapter_id: String,
	pub(crate) project: String,
	pub(crate) adapter_kind: String,
	pub(crate) evidence_class: String,
	pub(crate) docker_default: bool,
	pub(crate) host_global_installs_required: bool,
	pub(crate) overall_status: AdapterCoverageStatus,
	pub(crate) setup: AdapterExecutionEvidence,
	pub(crate) run: AdapterExecutionEvidence,
	pub(crate) result: AdapterExecutionEvidence,
	#[serde(default)]
	pub(crate) capabilities: Vec<AdapterCapabilityCoverage>,
	#[serde(default)]
	pub(crate) suites: Vec<AdapterSuiteCoverage>,
	#[serde(default)]
	pub(crate) scenarios: Vec<AdapterScenarioJudgment>,
	#[serde(default)]
	pub(crate) evidence: Vec<AdapterEvidencePointer>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) execution_metadata: Option<AdapterExecutionMetadata>,
	#[serde(default)]
	pub(crate) notes: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) follow_up: Option<FollowUpInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AdapterExecutionEvidence {
	pub(crate) status: AdapterCoverageStatus,
	pub(crate) evidence: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) command: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) artifact: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AdapterCapabilityCoverage {
	pub(crate) capability: String,
	pub(crate) status: AdapterCoverageStatus,
	pub(crate) evidence: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AdapterSuiteCoverage {
	pub(crate) suite_id: String,
	pub(crate) status: AdapterCoverageStatus,
	pub(crate) evidence: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AdapterScenarioJudgment {
	pub(crate) scenario_id: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) suite_id: Option<String>,
	pub(crate) status: AdapterCoverageStatus,
	pub(crate) elf_position: ElfScenarioPosition,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) comparison_outcome: Option<ScenarioComparisonOutcome>,
	pub(crate) evidence: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) command: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) artifact: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AdapterEvidencePointer {
	pub(crate) kind: String,
	#[serde(rename = "ref")]
	pub(crate) reference: String,
	pub(crate) status: AdapterCoverageStatus,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AdapterExecutionMetadata {
	#[serde(default)]
	pub(crate) sources: Vec<AdapterSource>,
	pub(crate) setup_path: String,
	pub(crate) runtime_boundary: String,
	pub(crate) resource_expectation: String,
	#[serde(default)]
	pub(crate) retry_guidance: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) research_depth: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AdapterSource {
	pub(crate) label: String,
	pub(crate) url: String,
	pub(crate) evidence: String,
}
