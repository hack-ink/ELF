mod details;
mod manifest;
mod misc;
mod summary;

pub(super) use self::{
	details::{
		AdapterScenarioJudgment, AdapterSource, AdapterSuiteCoverage, ExternalAdapterReport,
	},
	manifest::{ExternalAdapterManifest, ExternalAdapterSection, ExternalDockerIsolation},
	misc::{AdapterReport, CaptureIntegrationReport},
	summary::{
		AdapterStatusCounts, ExternalAdapterSummary, ScenarioOutcomeCounts, ScenarioPositionCounts,
	},
};
#[allow(unused_imports)]
pub(super) use details::{
	AdapterCapabilityCoverage, AdapterEvidencePointer, AdapterExecutionEvidence,
	AdapterExecutionMetadata,
};
