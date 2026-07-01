#![allow(clippy::single_component_path_imports, unused_crate_dependencies)]

//! Offline runner and publisher for real-world job benchmark fixtures.

mod artifacts;
mod cli;
mod commands;
mod diagnostic_reports;
mod enums;
mod external_adapter_reports;
mod external_adapters;
mod feature_metrics;
mod fixtures;
mod formatting;
mod job_reports;
mod markdown;
mod operational;
mod operational_reports;
mod quantitative;
mod quantitative_reports;
mod recovery;
mod report_root;
mod scoreboard;
mod scoreboard_reports;
mod scoring;
mod summary;
mod summary_reports;
mod validation;

use std::{
	collections::{BTreeMap, BTreeSet},
	fs,
	path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use color_eyre::{Result, eyre};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use artifacts::{
	AuthorityRecordCount, AuthorityRecoveryDrillArtifact, ConsolidationFixture,
	ConsolidationProposalFixture, CostReport, DerivedPageArtifact, DerivedPageRebuild,
	DerivedPageSection, MemorySummaryArtifact, MemorySummaryEntry, MemorySummarySourceTrace,
	ProactiveBriefArtifact, ProactiveSuggestion, ProducedAnswer, ProducedClaim, RecoveryBackupPitr,
	RecoveryDeadLetterHandling, RecoveryDegradedRead, RecoveryDrillTopology, RecoveryMeasurement,
	RecoveryMigrationRepair, RecoveryOutboxReplay, RecoveryQdrantRebuild,
	ScheduledMemoryExecutionTrace, ScheduledMemoryOutput, ScheduledMemoryTaskArtifact,
	WorkContinuityObserved, WorkJournalEntryArtifact, WorkJournalJanitorCandidateArtifact,
	WorkJournalNextStepArtifact, WorkJournalReadbackArtifact, WorkJournalRejectedOptionArtifact,
	WorkJournalWhereStoppedArtifact,
};
use cli::{
	Args, Command, ExportQuantitativeAuditManifestArgs, ExportQuantitativeProductManifestArgs,
	PublishArgs, RunArgs,
};
use diagnostic_reports::{
	OperatorDebugEvidence, OperatorUxGap, TraceExplainability, TraceStageExplainability,
};
use elf_cli::VERSION;
use enums::{
	AdapterCoverageStatus, ConsolidationReviewAction, CorpusProfile, ElfScenarioPosition,
	EvidenceLink, ExpectedClaim, ScenarioComparisonOutcome, TypedStatus,
};
use external_adapter_reports::{
	AdapterReport, AdapterScenarioJudgment, AdapterSource, AdapterStatusCounts,
	AdapterSuiteCoverage, CaptureIntegrationReport, ExternalAdapterManifest, ExternalAdapterReport,
	ExternalAdapterSection, ExternalAdapterSummary, ExternalDockerIsolation, ScenarioOutcomeCounts,
	ScenarioPositionCounts,
};
use external_adapters::{external_adapter_section, scenario_comparison_outcome};
use fixtures::{
	EvolutionConflict, FollowUpInput, MemoryEvolution, NegativeTrap, RealWorldJob,
	RequiredEvidence, TemporalValidity, UpdateRationale, WorkContinuityExpectation,
};
use job_reports::{
	ConsolidationExecutableGapReport, ConsolidationJobReport, ConsolidationProposalReport,
	DimensionScoreReport, EvolutionJobReport, EvolutionSummary, ExpectedEvidenceReport,
	FailureCounts, FollowUpReport, JobMetrics, JobReport, JobScoring, KnowledgeJobMetrics,
	MemorySummaryJobMetrics, PrivateCorpusRedaction, ProactiveBriefJobMetrics,
	RetrievalQualityReport, ScheduledMemoryJobMetrics, ScoreboardRankedMetrics,
	UnsupportedClaimReport, WorkContinuityJobMetrics,
};
use markdown::render_markdown;
use operational::operational_evidence_report;
use operational_reports::{
	OperationalAuthorityRecoveryReport, OperationalColdStartRestoreRebuild, OperationalCostSummary,
	OperationalEvidenceReport, OperationalEvidenceTierReport, OperationalLatencyReport,
	OperationalResourceSummary,
};
use quantitative::{
	QuantitativeReportInput, quantitative_audit_manifest_from_jobs,
	quantitative_product_manifest_from_report, quantitative_scoreboard_report,
};
use quantitative_reports::{
	QuantitativeAuditArtifact, QuantitativeAuditManifest, QuantitativeBenchmarkControls,
	QuantitativeBenchmarkReport, QuantitativeBenchmarkRow, QuantitativePerQueryRow,
	QuantitativeProductManifest,
};
use report_root::RealWorldReport;
use scoreboard::scoreboard_report;
use scoreboard_reports::{
	ScoreboardAnswerSafetyMetrics, ScoreboardCoverageMetrics, ScoreboardLifecycleMetrics,
	ScoreboardMetrics, ScoreboardOperationalMetrics, ScoreboardReport, ScoreboardRetrievalMetrics,
	ScoreboardRow,
};
use scoring::{job_report, score_job};
use summary::{evolution_summary, follow_up_reports, report_summary, suite_reports};
use summary_reports::{
	ConsolidationSummaryReport, KnowledgeSummary, MemorySummaryReport, ProactiveBriefSummaryReport,
	ReportSummary, ScheduledMemorySummaryReport, SuiteReport, WorkContinuitySummaryReport,
};
use validation::validate_job;

const JOB_SCHEMA: &str = "elf.real_world_job/v1";
const REPORT_SCHEMA: &str = "elf.real_world_job_report/v1";
const EXTERNAL_ADAPTER_MANIFEST_SCHEMA: &str = "elf.real_world_external_adapter_manifest/v1";
const EXTERNAL_ADAPTER_REPORT_SCHEMA: &str = "elf.real_world_external_adapter_report/v1";
const SCOREBOARD_SCHEMA: &str = "elf.quality_scoreboard/v1";
const OPERATIONAL_EVIDENCE_SCHEMA: &str = "elf.operational_evidence_gates/v1";
const AUTHORITY_RECOVERY_DRILL_SCHEMA: &str = "elf.authority_recovery_drill/v1";
const DEFAULT_FIXTURE_PATH: &str = "apps/elf-eval/fixtures/real_world_memory/work_resume";
const DEFAULT_REPORT_PATH: &str = "tmp/real-world-job/real-world-job-smoke-report.json";
const DEFAULT_MARKDOWN_PATH: &str = "tmp/real-world-job/real-world-job-smoke-report.md";
const DEFAULT_EXTERNAL_ADAPTER_MANIFEST_PATH: &str =
	"apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json";
const DEFAULT_RUN_ID: &str = "real-world-job-smoke";
const DEFAULT_ADAPTER_ID: &str = "fixture_smoke";
const DEFAULT_ADAPTER_NAME: &str = "ELF fixture smoke";
const DEFAULT_ADAPTER_BEHAVIOR: &str = "offline_fixture_response";
const DEFAULT_ADAPTER_STORAGE_STATUS: &str = "not_encoded";
const DEFAULT_ADAPTER_RUNTIME_STATUS: &str = "not_encoded";
const DEFAULT_ADAPTER_NOTES: &str = "Offline runner scores checked-in fixture responses; it does not exercise a live external adapter.";
const NOT_ENCODED_REASON: &str = "No checked-in real_world_job fixture is encoded for this suite.";
const FORBIDDEN_SOURCE_MUTATION_KEYS: [&str; 7] = [
	"delete_source",
	"delete_sources",
	"source_delete",
	"source_mutation",
	"source_mutations",
	"source_note_updates",
	"overwrite_source",
];
const SUITES: &[&str] = &[
	"trust_source_of_truth",
	"work_resume",
	"project_decisions",
	"retrieval",
	"memory_evolution",
	"adversarial_quality",
	"consolidation",
	"memory_summary",
	"proactive_brief",
	"scheduled_memory",
	"knowledge_compilation",
	"source_library",
	"operator_debugging_ux",
	"capture_integration",
	"work_continuity",
	"production_ops",
	"personalization",
	"core_archival_memory",
	"context_trajectory",
];
const SCOREBOARD_RESULT_STATES: &[&str] = &[
	"pass",
	"wrong_result",
	"incomplete",
	"blocked",
	"not_tested",
	"not_encoded",
	"not_comparable",
	"unsupported_claim",
];
const SCOREBOARD_EVIDENCE_CLASSES: &[&str] =
	&["fixture_backed", "live_baseline", "live_real_world", "research_gate"];
const SCOREBOARD_RETRIEVAL_K: usize = 5;

fn main() -> Result<()> {
	color_eyre::install()?;

	match Args::parse().command {
		Command::ExportQuantitativeAuditManifest(args) =>
			commands::export_quantitative_audit_manifest_command(args),
		Command::ExportQuantitativeProductManifest(args) =>
			commands::export_quantitative_product_manifest_command(args),
		Command::Run(args) => commands::run_command(args),
		Command::Publish(args) => commands::publish_command(args),
	}
}
