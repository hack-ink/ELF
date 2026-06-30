mod external_adapter_detail_reports;
mod external_adapter_manifest_reports;
mod external_adapter_misc_reports;
mod external_adapter_summary_reports;

pub(super) use self::{
	external_adapter_detail_reports::{
		AdapterScenarioJudgment, AdapterSource, AdapterSuiteCoverage, ExternalAdapterReport,
	},
	external_adapter_manifest_reports::{
		ExternalAdapterManifest, ExternalAdapterSection, ExternalDockerIsolation,
	},
	external_adapter_misc_reports::{AdapterReport, CaptureIntegrationReport},
	external_adapter_summary_reports::{
		AdapterStatusCounts, ExternalAdapterSummary, ScenarioOutcomeCounts, ScenarioPositionCounts,
	},
};
#[allow(unused_imports)]
pub(super) use external_adapter_detail_reports::{
	AdapterCapabilityCoverage, AdapterEvidencePointer, AdapterExecutionEvidence,
	AdapterExecutionMetadata,
};
