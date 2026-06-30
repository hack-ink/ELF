mod external_adapter_detail_reports;
mod external_adapter_manifest_reports;
mod external_adapter_misc_reports;
mod external_adapter_summary_reports;

#[allow(unused_imports)]
pub(super) use external_adapter_detail_reports::{
	AdapterCapabilityCoverage, AdapterEvidencePointer, AdapterExecutionEvidence,
	AdapterExecutionMetadata,
};
pub(super) use external_adapter_detail_reports::{
	AdapterScenarioJudgment, AdapterSource, AdapterSuiteCoverage, ExternalAdapterReport,
};
pub(super) use external_adapter_manifest_reports::{
	ExternalAdapterManifest, ExternalAdapterSection, ExternalDockerIsolation,
};
pub(super) use external_adapter_misc_reports::{AdapterReport, CaptureIntegrationReport};
pub(super) use external_adapter_summary_reports::{
	AdapterStatusCounts, ExternalAdapterSummary, ScenarioOutcomeCounts, ScenarioPositionCounts,
};
