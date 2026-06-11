#![allow(clippy::single_component_path_imports, unused_crate_dependencies)]

//! Offline runner and publisher for real-world job benchmark fixtures.

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

use elf_cli::VERSION;

const JOB_SCHEMA: &str = "elf.real_world_job/v1";
const REPORT_SCHEMA: &str = "elf.real_world_job_report/v1";
const EXTERNAL_ADAPTER_MANIFEST_SCHEMA: &str = "elf.real_world_external_adapter_manifest/v1";
const EXTERNAL_ADAPTER_REPORT_SCHEMA: &str = "elf.real_world_external_adapter_report/v1";
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
	"consolidation",
	"knowledge_compilation",
	"operator_debugging_ux",
	"capture_integration",
	"production_ops",
	"personalization",
	"core_archival_memory",
];

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
struct Args {
	#[command(subcommand)]
	command: Command,
}

#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab")]
enum Command {
	/// Parse and score real_world_job fixtures, then emit a JSON report.
	Run(RunArgs),
	/// Render Markdown from a generated real_world_job JSON report.
	Publish(PublishArgs),
}

#[derive(Debug, Parser)]
struct RunArgs {
	/// Fixture file or directory containing real_world_job JSON fixtures.
	#[arg(long, value_name = "PATH", default_value = DEFAULT_FIXTURE_PATH)]
	fixtures: PathBuf,
	/// Write report JSON to this file. Omit to print to stdout.
	#[arg(long, value_name = "FILE")]
	out: Option<PathBuf>,
	/// Stable run id recorded in the generated report.
	#[arg(long, default_value = DEFAULT_RUN_ID)]
	run_id: String,
	/// Adapter id recorded for the offline smoke response.
	#[arg(long, default_value = DEFAULT_ADAPTER_ID)]
	adapter_id: String,
	/// Human-readable adapter name recorded in the generated report.
	#[arg(long, default_value = DEFAULT_ADAPTER_NAME)]
	adapter_name: String,
	/// Adapter behavior label recorded in the generated report.
	#[arg(long, default_value = DEFAULT_ADAPTER_BEHAVIOR)]
	adapter_behavior: String,
	/// Adapter storage typed status recorded in the generated report.
	#[arg(long, default_value = DEFAULT_ADAPTER_STORAGE_STATUS)]
	adapter_storage_status: String,
	/// Adapter runtime typed status recorded in the generated report.
	#[arg(long, default_value = DEFAULT_ADAPTER_RUNTIME_STATUS)]
	adapter_runtime_status: String,
	/// Adapter notes recorded in the generated report.
	#[arg(long, default_value = DEFAULT_ADAPTER_NOTES)]
	adapter_notes: String,
	/// Real-world external adapter manifest to include in report coverage.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_EXTERNAL_ADAPTER_MANIFEST_PATH)]
	external_adapter_manifest: PathBuf,
	/// Skip loading the real-world external adapter coverage manifest.
	#[arg(long)]
	skip_external_adapter_manifest: bool,
}

#[derive(Debug, Parser)]
struct PublishArgs {
	/// Generated real_world_job JSON report.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_REPORT_PATH)]
	report: PathBuf,
	/// Write Markdown to this file. Omit to print to stdout.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_MARKDOWN_PATH)]
	out: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct RealWorldJob {
	schema: String,
	job_id: String,
	suite: String,
	title: String,
	corpus: Corpus,
	#[serde(default)]
	timeline: Vec<TimelineEvent>,
	prompt: Prompt,
	expected_answer: ExpectedAnswer,
	#[serde(default)]
	required_evidence: Vec<RequiredEvidence>,
	#[serde(default)]
	negative_traps: Vec<NegativeTrap>,
	scoring_rubric: ScoringRubric,
	allowed_uncertainty: AllowedUncertainty,
	operator_debug: Option<OperatorDebugEvidence>,
	#[serde(default)]
	tags: Vec<String>,
	#[serde(default)]
	encoding: JobEncoding,
	memory_evolution: Option<MemoryEvolution>,
}

#[derive(Debug, Deserialize)]
struct Corpus {
	corpus_id: String,
	profile: CorpusProfile,
	#[serde(default)]
	items: Vec<CorpusItem>,
	#[serde(default)]
	capture_behaviors: CaptureIntegrationReport,

	adapter_response: Option<AdapterResponse>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum CorpusProfile {
	Synthetic,
	PrivateSanitized,
	GeneratedPublic,
	ExternalAdapter,
}
impl CorpusProfile {
	fn as_str(&self) -> &'static str {
		match self {
			Self::Synthetic => "synthetic",
			Self::PrivateSanitized => "private_sanitized",
			Self::GeneratedPublic => "generated_public",
			Self::ExternalAdapter => "external_adapter",
		}
	}
}

#[derive(Debug, Deserialize)]
struct CorpusItem {
	evidence_id: String,
	kind: String,

	text: Option<String>,

	local_ref: Option<String>,
	#[serde(default)]
	source_ref: Value,

	created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TimelineEvent {
	event_id: String,
	ts: String,
	actor: String,
	action: String,
	#[serde(default)]
	evidence_ids: Vec<String>,
	summary: String,
}

#[derive(Debug, Deserialize)]
struct Prompt {
	role: String,
	content: String,
	job_mode: String,
	#[serde(default)]
	constraints: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedAnswer {
	#[serde(default)]
	must_include: Vec<ExpectedClaim>,
	#[serde(default)]
	must_not_include: Vec<String>,
	#[serde(default)]
	evidence_links: BTreeMap<String, EvidenceLink>,
	answer_type: String,
	#[serde(default)]
	accepted_alternates: Vec<Value>,
	#[serde(default)]
	requires_caveat: bool,
	#[serde(default)]
	requires_refusal: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum ExpectedClaim {
	Text(String),
	Object { claim_id: Option<String>, text: String },
}
impl ExpectedClaim {
	fn claim_id(&self) -> Option<&str> {
		match self {
			Self::Text(_) => None,
			Self::Object { claim_id, .. } => claim_id.as_deref(),
		}
	}

	fn text(&self) -> &str {
		match self {
			Self::Text(text) => text,
			Self::Object { text, .. } => text,
		}
	}
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum EvidenceLink {
	One(String),
	Many(Vec<String>),
}
impl EvidenceLink {
	fn ids(&self) -> BTreeSet<String> {
		match self {
			Self::One(id) => BTreeSet::from([id.clone()]),
			Self::Many(ids) => ids.iter().cloned().collect(),
		}
	}
}

#[derive(Debug, Deserialize)]
struct RequiredEvidence {
	evidence_id: String,
	claim_id: String,
	requirement: String,

	quote: Option<String>,

	selector: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NegativeTrap {
	trap_id: String,
	#[serde(rename = "type")]
	trap_type: String,
	#[serde(default)]
	evidence_ids: Vec<String>,
	#[serde(default)]
	failure_if_used: bool,
}

#[derive(Debug, Default, Deserialize)]
struct JobEncoding {
	status: Option<TypedStatus>,
	reason: Option<String>,
	follow_up: Option<FollowUpInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct FollowUpInput {
	title: String,
	reason: String,
}

#[derive(Debug, Deserialize)]
struct MemoryEvolution {
	#[serde(default)]
	current_evidence_ids: Vec<String>,
	#[serde(default)]
	historical_evidence_ids: Vec<String>,
	#[serde(default)]
	stale_trap_ids: Vec<String>,
	#[serde(default)]
	conflicts: Vec<EvolutionConflict>,
	update_rationale: Option<UpdateRationale>,
	temporal_validity: Option<TemporalValidity>,
	history_readback: Option<HistoryReadback>,
}

#[derive(Debug, Deserialize)]
struct EvolutionConflict {
	conflict_id: String,
	claim_id: String,
	current_evidence_id: String,
	historical_evidence_id: String,
	resolved_by_evidence_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateRationale {
	claim_id: String,
	#[serde(default)]
	evidence_ids: Vec<String>,
	available: bool,
}

#[derive(Debug, Deserialize)]
struct TemporalValidity {
	required: bool,
	encoded: bool,
	follow_up: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HistoryReadback {
	encoded: bool,
	#[serde(default)]
	required_event_types: Vec<String>,
	requires_note_version_links: bool,
}

#[derive(Debug, Deserialize)]
struct ScoringRubric {
	#[serde(default)]
	dimensions: BTreeMap<String, RubricDimension>,
	pass_threshold: f64,
	#[serde(default)]
	hard_fail_rules: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RubricDimension {
	weight: f64,
	max_points: f64,
	criteria: Value,
}

#[derive(Debug, Deserialize)]
struct AllowedUncertainty {
	can_answer_unknown: bool,
	#[serde(default)]
	acceptable_phrases: Vec<String>,
	fallback_action: String,
}

#[derive(Clone, Debug, Deserialize)]
struct AdapterResponse {
	adapter_id: Option<String>,
	answer: ProducedAnswer,
	consolidation: Option<ConsolidationFixture>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ProducedAnswer {
	content: String,
	#[serde(default)]
	claims: Vec<ProducedClaim>,
	#[serde(default)]
	evidence_ids: Vec<String>,
	#[serde(default)]
	pages: Vec<DerivedPageArtifact>,
	#[serde(skip_serializing_if = "Option::is_none")]
	latency_ms: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	cost: Option<CostReport>,
	#[serde(skip_serializing_if = "Option::is_none")]
	trace_explainability: Option<TraceExplainability>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ProducedClaim {
	#[serde(skip_serializing_if = "Option::is_none")]
	claim_id: Option<String>,
	text: String,
	#[serde(default)]
	evidence_ids: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	confidence: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DerivedPageArtifact {
	page_id: String,
	page_type: String,
	title: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	path: Option<String>,
	#[serde(default)]
	sections: Vec<DerivedPageSection>,
	#[serde(default)]
	backlinks: Vec<String>,
	#[serde(default)]
	lint_findings: Vec<DerivedPageLintFinding>,
	#[serde(skip_serializing_if = "Option::is_none")]
	rebuild: Option<DerivedPageRebuild>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DerivedPageSection {
	section_id: String,
	heading: String,
	role: String,
	content: String,
	#[serde(default)]
	evidence_ids: Vec<String>,
	#[serde(default)]
	timeline_event_ids: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	unsupported_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DerivedPageLintFinding {
	finding_id: String,
	finding_type: String,
	severity: String,
	text: String,
	#[serde(default)]
	evidence_ids: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	trap_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DerivedPageRebuild {
	first_hash: String,
	second_hash: String,
	deterministic: bool,
	#[serde(default)]
	allowed_variance: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct ConsolidationFixture {
	#[serde(default)]
	proposals: Vec<ConsolidationProposalFixture>,
	#[serde(default)]
	executable_gaps: Vec<ConsolidationExecutableGap>,
}

#[derive(Clone, Debug, Deserialize)]
struct ConsolidationProposalFixture {
	proposal_id: String,
	proposal_kind: String,
	#[serde(default)]
	source_refs: Vec<String>,
	#[serde(default)]
	expected_source_refs: Vec<String>,
	usefulness_score: f64,
	min_usefulness_score: f64,
	expected_review_action: ConsolidationReviewAction,
	actual_review_action: ConsolidationReviewAction,
	#[serde(default)]
	source_mutations: Vec<Value>,
	#[serde(default)]
	unsupported_claim_count: usize,
	#[serde(default)]
	unsupported_claim_flags: Vec<Value>,
	#[serde(default)]
	diff: Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ConsolidationReviewAction {
	Apply,
	Discard,
	Defer,
}

#[derive(Clone, Debug, Deserialize)]
struct ConsolidationExecutableGap {
	primitive: String,
	follow_up_issue: String,
	reason: String,
	#[serde(default)]
	blocks_fixture_pass: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CostReport {
	#[serde(skip_serializing_if = "Option::is_none")]
	currency: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	amount: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	input_tokens: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	output_tokens: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct OperatorDebugEvidence {
	failure_mode: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	trace_id: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	viewer_url: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	admin_trace_bundle_url: Option<String>,
	root_cause: String,
	steps_to_root_cause: u32,
	raw_sql_needed: bool,
	dropped_candidate_visibility: String,
	trace_completeness: String,
	repair_action_clarity: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	trace_available: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	replay_command_available: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	replay_command: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	replay_artifact: Option<String>,
	#[serde(default)]
	viewer_panels: Vec<String>,
	#[serde(default)]
	cli_steps: Vec<String>,
	#[serde(default)]
	trace_evidence: Vec<String>,
	#[serde(default)]
	ux_gaps: Vec<OperatorUxGap>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct OperatorUxGap {
	gap_id: String,
	severity: String,
	description: String,
	follow_up_issue: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TraceExplainability {
	#[serde(skip_serializing_if = "Option::is_none")]
	trace_id: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	failure_stage: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	failure_reason: Option<String>,
	#[serde(default)]
	stages: Vec<TraceStageExplainability>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TraceStageExplainability {
	stage_name: String,
	#[serde(default)]
	kept_evidence: Vec<String>,
	#[serde(default)]
	dropped_evidence: Vec<String>,
	#[serde(default)]
	demoted_evidence: Vec<String>,
	#[serde(default)]
	distractor_evidence: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	notes: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum TypedStatus {
	Pass,
	WrongResult,
	LifecycleFail,
	Incomplete,
	Blocked,
	NotEncoded,
	UnsupportedClaim,
}

#[derive(Debug, Deserialize, Serialize)]
struct RealWorldReport {
	schema: String,
	run_id: String,
	generated_at: String,
	runner_version: String,
	corpus_profile: String,
	adapter: AdapterReport,
	#[serde(default)]
	external_adapters: ExternalAdapterSection,
	capture_integration: CaptureIntegrationReport,
	summary: ReportSummary,
	suites: Vec<SuiteReport>,
	jobs: Vec<JobReport>,
	unsupported_claims: Vec<UnsupportedClaimReport>,
	not_encoded_suites: Vec<String>,
	private_corpus_redaction: PrivateCorpusRedaction,
	#[serde(default)]
	evolution: EvolutionSummary,
	#[serde(default)]
	follow_ups: Vec<FollowUpReport>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AdapterReport {
	adapter_id: String,
	name: String,
	behavior: String,
	storage: TypedStatus,
	runtime: TypedStatus,
	notes: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum AdapterCoverageStatus {
	Real,
	Mocked,
	Unsupported,
	Blocked,
	Incomplete,
	WrongResult,
	LifecycleFail,
	Pass,
	NotEncoded,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ElfScenarioPosition {
	Wins,
	Ties,
	Loses,
	Untested,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ScenarioComparisonOutcome {
	Win,
	Tie,
	Loss,
	NotTested,
	Blocked,
	NonGoal,
}

#[derive(Debug, Deserialize)]
struct ExternalAdapterManifest {
	schema: String,
	manifest_id: String,
	docker_isolation: ExternalDockerIsolation,
	#[serde(default)]
	adapters: Vec<ExternalAdapterReport>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ExternalAdapterSection {
	schema: String,
	manifest_id: String,
	docker_isolation: ExternalDockerIsolation,
	summary: ExternalAdapterSummary,
	#[serde(default)]
	adapters: Vec<ExternalAdapterReport>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ExternalDockerIsolation {
	default: bool,
	compose_file: String,
	runner: String,
	artifact_dir: String,
	host_global_installs_required: bool,
	#[serde(default)]
	notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ExternalAdapterReport {
	adapter_id: String,
	project: String,
	adapter_kind: String,
	evidence_class: String,
	docker_default: bool,
	host_global_installs_required: bool,
	overall_status: AdapterCoverageStatus,
	setup: AdapterExecutionEvidence,
	run: AdapterExecutionEvidence,
	result: AdapterExecutionEvidence,
	#[serde(default)]
	capabilities: Vec<AdapterCapabilityCoverage>,
	#[serde(default)]
	suites: Vec<AdapterSuiteCoverage>,
	#[serde(default)]
	scenarios: Vec<AdapterScenarioJudgment>,
	#[serde(default)]
	evidence: Vec<AdapterEvidencePointer>,
	#[serde(skip_serializing_if = "Option::is_none")]
	execution_metadata: Option<AdapterExecutionMetadata>,
	#[serde(default)]
	notes: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	follow_up: Option<FollowUpInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AdapterExecutionEvidence {
	status: AdapterCoverageStatus,
	evidence: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	command: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	artifact: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AdapterCapabilityCoverage {
	capability: String,
	status: AdapterCoverageStatus,
	evidence: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AdapterSuiteCoverage {
	suite_id: String,
	status: AdapterCoverageStatus,
	evidence: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AdapterScenarioJudgment {
	scenario_id: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	suite_id: Option<String>,
	status: AdapterCoverageStatus,
	elf_position: ElfScenarioPosition,
	#[serde(skip_serializing_if = "Option::is_none")]
	comparison_outcome: Option<ScenarioComparisonOutcome>,
	evidence: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	command: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	artifact: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AdapterEvidencePointer {
	kind: String,
	#[serde(rename = "ref")]
	reference: String,
	status: AdapterCoverageStatus,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AdapterExecutionMetadata {
	#[serde(default)]
	sources: Vec<AdapterSource>,
	setup_path: String,
	runtime_boundary: String,
	resource_expectation: String,
	#[serde(default)]
	retry_guidance: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	research_depth: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AdapterSource {
	label: String,
	url: String,
	evidence: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ExternalAdapterSummary {
	adapter_count: usize,
	external_project_count: usize,
	docker_default_count: usize,
	host_global_install_required_count: usize,
	fixture_backed_count: usize,
	live_baseline_only_count: usize,
	live_real_world_count: usize,
	#[serde(default)]
	research_gate_count: usize,
	overall_status_counts: AdapterStatusCounts,
	capability_status_counts: AdapterStatusCounts,
	suite_status_counts: AdapterStatusCounts,
	#[serde(default)]
	scenario_status_counts: AdapterStatusCounts,
	#[serde(default)]
	scenario_position_counts: ScenarioPositionCounts,
	#[serde(default)]
	scenario_outcome_counts: ScenarioOutcomeCounts,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct AdapterStatusCounts {
	real: usize,
	mocked: usize,
	unsupported: usize,
	blocked: usize,
	incomplete: usize,
	wrong_result: usize,
	lifecycle_fail: usize,
	pass: usize,
	not_encoded: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ScenarioPositionCounts {
	wins: usize,
	ties: usize,
	loses: usize,
	untested: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ScenarioOutcomeCounts {
	win: usize,
	tie: usize,
	loss: usize,
	not_tested: usize,
	blocked: usize,
	non_goal: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct CaptureIntegrationReport {
	#[serde(default)]
	real: Vec<String>,
	#[serde(default)]
	fixture_backed: Vec<String>,
	#[serde(default)]
	mocked: Vec<String>,
	#[serde(default)]
	blocked: Vec<String>,
	#[serde(default)]
	not_encoded: Vec<String>,
	#[serde(default)]
	notes: Vec<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ReportSummary {
	job_count: usize,
	encoded_suite_count: usize,
	pass: usize,
	wrong_result: usize,
	lifecycle_fail: usize,
	incomplete: usize,
	blocked: usize,
	not_encoded: usize,
	unsupported_claim: usize,
	unsupported_claim_count: usize,
	wrong_result_count: usize,
	#[serde(default)]
	stale_answer_count: usize,
	#[serde(default)]
	conflict_detection_count: usize,
	#[serde(default)]
	update_rationale_available_count: usize,
	#[serde(default)]
	temporal_validity_not_encoded_count: usize,
	#[serde(default)]
	history_readback_encoded_count: usize,
	expected_evidence_total: usize,
	expected_evidence_matched: usize,
	expected_evidence_recall: f64,
	irrelevant_context_count: usize,
	irrelevant_context_ratio: f64,
	trace_explainability_count: usize,
	wrong_result_stage_attribution_count: usize,
	mean_score: f64,
	mean_latency_ms: Option<f64>,
	total_cost: Option<CostReport>,
	#[serde(default)]
	evidence_required_count: usize,
	#[serde(default)]
	evidence_covered_count: usize,
	#[serde(default)]
	evidence_coverage: f64,
	#[serde(default)]
	source_ref_required_count: usize,
	#[serde(default)]
	source_ref_covered_count: usize,
	#[serde(default)]
	source_ref_coverage: f64,
	#[serde(default)]
	quote_required_count: usize,
	#[serde(default)]
	quote_covered_count: usize,
	#[serde(default)]
	quote_coverage: f64,
	#[serde(default)]
	stale_retrieval_count: usize,
	#[serde(default)]
	scope_check_count: usize,
	#[serde(default)]
	scope_correct_count: usize,
	#[serde(default)]
	scope_correctness: f64,
	#[serde(default)]
	scope_violation_count: usize,
	#[serde(default)]
	redaction_leak_count: usize,
	#[serde(default)]
	qdrant_rebuild_case_count: usize,
	#[serde(default)]
	qdrant_rebuild_pass_count: usize,
	#[serde(default)]
	operator_debug_job_count: usize,
	#[serde(default)]
	raw_sql_needed_count: usize,
	#[serde(default)]
	trace_incomplete_count: usize,
	#[serde(default)]
	operator_ux_gap_count: usize,
	#[serde(default)]
	consolidation: ConsolidationSummaryReport,
	#[serde(skip_serializing_if = "Option::is_none")]
	knowledge: Option<KnowledgeSummary>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ConsolidationSummaryReport {
	proposal_count: usize,
	proposal_usefulness: Option<f64>,
	lineage_completeness: Option<f64>,
	review_action_correctness: Option<f64>,
	source_mutation_count: usize,
	proposal_unsupported_claim_count: usize,
	executable_gap_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct KnowledgeSummary {
	job_count: usize,
	page_count: usize,
	section_count: usize,
	backlink_count: usize,
	pages_with_backlinks: usize,
	citation_coverage: f64,
	stale_claim_detection: f64,
	rebuild_determinism: f64,
	backlink_coverage: f64,
	page_usefulness: f64,
	unsupported_summary_count: usize,
	untraced_section_count: usize,
	allowed_variance_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct SuiteReport {
	suite_id: String,
	status: TypedStatus,
	encoded_job_count: usize,
	score_mean: Option<f64>,
	unsupported_claim_count: usize,
	wrong_result_count: usize,
	#[serde(default)]
	stale_answer_count: usize,
	#[serde(default)]
	conflict_detection_count: usize,
	#[serde(default)]
	update_rationale_available_count: usize,
	#[serde(default)]
	temporal_validity_not_encoded_count: usize,
	#[serde(default)]
	history_readback_encoded_count: usize,
	expected_evidence_recall: Option<f64>,
	irrelevant_context_ratio: Option<f64>,
	trace_explainability_count: usize,
	reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct JobReport {
	suite_id: String,
	job_id: String,
	title: String,
	status: TypedStatus,
	answer_type: String,
	requires_caveat: bool,
	requires_refusal: bool,
	can_answer_unknown: bool,
	normalized_score: f64,
	hard_fail_hits: Vec<String>,
	expected_evidence: Vec<ExpectedEvidenceReport>,
	produced_answer: String,
	produced_evidence: Vec<String>,
	unsupported_claim_count: usize,
	wrong_result_count: usize,
	#[serde(default)]
	stale_answer_count: usize,
	#[serde(default)]
	conflict_detection_count: usize,
	#[serde(default)]
	update_rationale_available: bool,
	#[serde(default)]
	temporal_validity_not_encoded: bool,
	#[serde(default)]
	history_readback_encoded: bool,
	retrieval_quality: RetrievalQualityReport,
	latency_ms: Option<f64>,
	cost: Option<CostReport>,
	trace_explainability: Option<TraceExplainability>,
	#[serde(skip_serializing_if = "Option::is_none")]
	knowledge: Option<KnowledgeJobMetrics>,
	trap_ids_used: Vec<String>,
	dimension_scores: Vec<DimensionScoreReport>,
	reason: String,
	#[serde(default)]
	evidence_required_count: usize,
	#[serde(default)]
	evidence_covered_count: usize,
	#[serde(default)]
	source_ref_required_count: usize,
	#[serde(default)]
	source_ref_covered_count: usize,
	#[serde(default)]
	quote_required_count: usize,
	#[serde(default)]
	quote_covered_count: usize,
	#[serde(default)]
	stale_retrieval_count: usize,
	#[serde(default)]
	scope_check_count: usize,
	#[serde(default)]
	scope_correct_count: usize,
	#[serde(default)]
	scope_violation_count: usize,
	#[serde(default)]
	redaction_leak_count: usize,
	#[serde(default)]
	qdrant_rebuild_case: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	operator_debug: Option<OperatorDebugEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	evolution: Option<EvolutionJobReport>,
	#[serde(skip_serializing_if = "Option::is_none")]
	consolidation: Option<ConsolidationJobReport>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ExpectedEvidenceReport {
	evidence_id: String,
	claim_id: String,
	requirement: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct DimensionScoreReport {
	dimension: String,
	score: f64,
	max_points: f64,
	weight: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct RetrievalQualityReport {
	expected_evidence_total: usize,
	expected_evidence_matched: usize,
	expected_evidence_recall: f64,
	produced_evidence_total: usize,
	irrelevant_context_count: usize,
	irrelevant_context_ratio: f64,
	trap_context_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ConsolidationJobReport {
	proposal_count: usize,
	proposal_usefulness: Option<f64>,
	lineage_completeness: Option<f64>,
	review_action_correctness: Option<f64>,
	source_mutation_count: usize,
	proposal_unsupported_claim_count: usize,
	executable_gaps: Vec<ConsolidationExecutableGapReport>,
	proposals: Vec<ConsolidationProposalReport>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ConsolidationProposalReport {
	proposal_id: String,
	proposal_kind: String,
	usefulness_score: f64,
	min_usefulness_score: f64,
	lineage_completeness: f64,
	expected_review_action: ConsolidationReviewAction,
	actual_review_action: ConsolidationReviewAction,
	review_action_correct: bool,
	source_mutation_count: usize,
	unsupported_claim_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ConsolidationExecutableGapReport {
	primitive: String,
	follow_up_issue: String,
	reason: String,
	blocks_fixture_pass: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct UnsupportedClaimReport {
	suite_id: String,
	job_id: String,
	claim_id: Option<String>,
	claim_text: String,
	reason: String,
	evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct KnowledgeJobMetrics {
	page_count: usize,
	section_count: usize,
	traced_section_count: usize,
	flagged_unsupported_section_count: usize,
	untraced_section_count: usize,
	unsupported_summary_count: usize,
	backlink_count: usize,
	pages_with_backlinks: usize,
	stale_trap_count: usize,
	stale_traps_detected: usize,
	rebuild_page_count: usize,
	deterministic_rebuild_count: usize,
	rebuild_failure_count: usize,
	allowed_variance_count: usize,
	citation_coverage: f64,
	stale_claim_detection: f64,
	rebuild_determinism: f64,
	backlink_coverage: f64,
	page_usefulness: f64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct EvolutionSummary {
	stale_answer_count: usize,
	conflict_detection_count: usize,
	update_rationale_available_count: usize,
	temporal_validity_not_encoded_count: usize,
	history_readback_encoded_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EvolutionJobReport {
	current_evidence: Vec<String>,
	historical_evidence: Vec<String>,
	stale_trap_ids_used: Vec<String>,
	stale_answer_count: usize,
	conflict_count: usize,
	conflict_detection_count: usize,
	update_rationale_available: bool,
	temporal_validity_required: bool,
	temporal_validity_encoded: bool,
	temporal_validity_not_encoded: bool,
	history_readback_encoded: bool,
	history_event_types: Vec<String>,
	history_requires_note_version_links: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	follow_up: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct FollowUpReport {
	suite_id: String,
	job_id: String,
	title: String,
	reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct PrivateCorpusRedaction {
	policy: String,
	private_fixture_count: usize,
}

#[derive(Debug)]
struct JobScoring {
	status: TypedStatus,
	normalized_score: f64,
	hard_fail_hits: Vec<String>,
	unsupported_claims: Vec<UnsupportedClaimReport>,
	wrong_result_count: usize,
	knowledge: Option<KnowledgeJobMetrics>,
	trap_ids_used: Vec<String>,
	dimension_scores: Vec<DimensionScoreReport>,
	reason: String,
	evolution: Option<EvolutionJobReport>,
	consolidation: Option<ConsolidationJobReport>,
}

#[derive(Debug, Default)]
struct FailureCounts {
	missing_claims: usize,
	forbidden_claims: usize,
	missing_evidence: usize,
	trap_uses: usize,
	unsupported_claims: usize,
	operator_debug_missing: usize,
	operator_debug_raw_sql: usize,
	operator_debug_trace_gaps: usize,
	operator_debug_repair_unclear: usize,
	stale_answers: usize,
	conflict_detection_missing: usize,
	update_rationale_missing: usize,
	latency_violations: usize,
	proposal_usefulness_failures: usize,
	lineage_failures: usize,
	review_action_failures: usize,
	source_mutations: usize,
	blocking_executable_gaps: usize,
	untraced_page_sections: usize,
	missed_stale_findings: usize,
	rebuild_failures: usize,
	page_usefulness_failures: usize,
}

#[derive(Debug, Default)]
struct JobMetrics {
	evidence_required_count: usize,
	evidence_covered_count: usize,
	source_ref_required_count: usize,
	source_ref_covered_count: usize,
	quote_required_count: usize,
	quote_covered_count: usize,
	stale_retrieval_count: usize,
	scope_check_count: usize,
	scope_correct_count: usize,
	scope_violation_count: usize,
	redaction_leak_count: usize,
	qdrant_rebuild_case: bool,
}

fn main() -> Result<()> {
	color_eyre::install()?;

	match Args::parse().command {
		Command::Run(args) => run_command(args),
		Command::Publish(args) => publish_command(args),
	}
}

fn run_command(args: RunArgs) -> Result<()> {
	let jobs = load_jobs(&args.fixtures)?;
	let report = build_report(&jobs, &args)?;
	let json = serde_json::to_string_pretty(&report)?;

	write_or_print(args.out.as_deref(), json.as_str())
}

fn publish_command(args: PublishArgs) -> Result<()> {
	let raw = fs::read_to_string(&args.report)?;
	let report = serde_json::from_str::<RealWorldReport>(&raw)?;
	let markdown = render_markdown(&report, &args.report);

	write_or_print(args.out.as_deref(), markdown.as_str())
}

fn load_jobs(path: &Path) -> Result<Vec<RealWorldJob>> {
	let paths = fixture_paths(path)?;
	let mut jobs = Vec::with_capacity(paths.len());

	for fixture in paths {
		let raw = fs::read_to_string(&fixture)?;
		let job = serde_json::from_str::<RealWorldJob>(&raw)
			.map_err(|err| eyre::eyre!("Failed to parse {}: {err}", fixture.display()))?;

		validate_job(&job, &fixture)?;

		jobs.push(job);
	}

	Ok(jobs)
}

fn fixture_paths(path: &Path) -> Result<Vec<PathBuf>> {
	if path.is_file() {
		return Ok(vec![path.to_path_buf()]);
	}
	if !path.is_dir() {
		return Err(eyre::eyre!("Fixture path does not exist: {}", path.display()));
	}

	let mut paths = Vec::new();

	collect_fixture_paths(path, &mut paths)?;

	paths.sort();

	if paths.is_empty() {
		return Err(eyre::eyre!("No JSON fixtures found in {}.", path.display()));
	}

	Ok(paths)
}

fn collect_fixture_paths(path: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
	for entry in fs::read_dir(path)? {
		let entry = entry?;
		let entry_path = entry.path();

		if entry_path.is_dir() {
			collect_fixture_paths(entry_path.as_path(), paths)?;
		} else if entry_path.extension().and_then(|ext| ext.to_str()) == Some("json") {
			paths.push(entry_path);
		}
	}

	Ok(())
}

fn validate_job(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.schema != JOB_SCHEMA {
		return Err(eyre::eyre!(
			"{} has schema {}, expected {JOB_SCHEMA}.",
			path.display(),
			job.schema
		));
	}

	validate_job_identity(job, path)?;

	if !SUITES.contains(&job.suite.as_str()) {
		return Err(eyre::eyre!("{} uses unknown suite {}.", path.display(), job.suite));
	}

	validate_corpus_items(job, path)?;
	validate_timeline(job, path)?;
	validate_prompt(job, path)?;
	validate_expected_answer(job, path)?;
	validate_required_evidence(job, path)?;
	validate_consolidation_fixture(job, path)?;
	validate_adapter_response(job, path)?;
	validate_scoring_rubric(job, path)?;
	validate_allowed_uncertainty(job, path)?;
	validate_operator_debug(job, path)?;
	validate_job_encoding(job, path)?;
	validate_memory_evolution(job, path)?;
	validate_trace_explainability(job, path)?;

	Ok(())
}

fn validate_job_identity(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.job_id.trim().is_empty()
		|| job.suite.trim().is_empty()
		|| job.title.trim().is_empty()
		|| job.corpus.corpus_id.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete job identity.", path.display()));
	}

	for tag in &job.tags {
		if tag.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty tag.", path.display()));
		}
	}

	if let Some(adapter_response) = &job.corpus.adapter_response
		&& adapter_response.adapter_id.as_deref().is_some_and(str::is_empty)
	{
		return Err(eyre::eyre!("{} has an empty adapter_response adapter_id.", path.display()));
	}

	Ok(())
}

fn validate_corpus_items(job: &RealWorldJob, path: &Path) -> Result<()> {
	let mut evidence_ids = BTreeSet::new();

	for item in &job.corpus.items {
		if item.evidence_id.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} has a corpus item with an empty evidence_id.",
				path.display()
			));
		}
		if item.kind.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} has corpus item {} with an empty kind.",
				path.display(),
				item.evidence_id
			));
		}
		if item.text.is_none() && item.local_ref.is_none() {
			return Err(eyre::eyre!(
				"{} corpus item {} must provide text or local_ref.",
				path.display(),
				item.evidence_id
			));
		}
		if !item.source_ref.is_object() {
			return Err(eyre::eyre!(
				"{} corpus item {} must provide an object source_ref.",
				path.display(),
				item.evidence_id
			));
		}

		if let Some(created_at) = &item.created_at {
			validate_optional_rfc3339(created_at, path, item.evidence_id.as_str())?;
		}

		evidence_ids.insert(item.evidence_id.clone());
	}
	for trap in &job.negative_traps {
		if trap.trap_id.trim().is_empty() || trap.trap_type.trim().is_empty() {
			return Err(eyre::eyre!("{} has an incomplete negative trap.", path.display()));
		}

		for evidence_id in &trap.evidence_ids {
			ensure_known_evidence(path, &evidence_ids, evidence_id)?;
		}
	}

	Ok(())
}

fn validate_timeline(job: &RealWorldJob, path: &Path) -> Result<()> {
	let evidence_ids = corpus_evidence_ids(job);

	for event in &job.timeline {
		if event.event_id.trim().is_empty()
			|| event.actor.trim().is_empty()
			|| event.action.trim().is_empty()
			|| event.summary.trim().is_empty()
		{
			return Err(eyre::eyre!("{} has an incomplete timeline event.", path.display()));
		}

		validate_required_rfc3339(event.ts.as_str(), path, event.event_id.as_str())?;

		for evidence_id in &event.evidence_ids {
			ensure_known_evidence(path, &evidence_ids, evidence_id)?;
		}
	}

	Ok(())
}

fn validate_prompt(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.prompt.role.trim().is_empty()
		|| job.prompt.content.trim().is_empty()
		|| job.prompt.job_mode.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete prompt.", path.display()));
	}

	for constraint in &job.prompt.constraints {
		if constraint.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty prompt constraint.", path.display()));
		}
	}

	Ok(())
}

fn validate_expected_answer(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.expected_answer.answer_type.trim().is_empty() {
		return Err(eyre::eyre!("{} has an empty expected answer type.", path.display()));
	}

	for claim in &job.expected_answer.must_include {
		if claim.text().trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty expected claim.", path.display()));
		}
	}
	for claim in &job.expected_answer.must_not_include {
		if claim.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty forbidden claim.", path.display()));
		}
	}
	for phrase in &job.expected_answer.accepted_alternates {
		if phrase.is_null() {
			return Err(eyre::eyre!("{} has a null accepted alternate.", path.display()));
		}
	}

	Ok(())
}

fn validate_required_evidence(job: &RealWorldJob, path: &Path) -> Result<()> {
	let evidence_ids = corpus_evidence_ids(job);
	let corpus_text = corpus_text_by_id(job);

	for evidence in &job.required_evidence {
		if evidence.claim_id.trim().is_empty() || evidence.requirement.trim().is_empty() {
			return Err(eyre::eyre!("{} has incomplete required evidence.", path.display()));
		}

		ensure_known_evidence(path, &evidence_ids, evidence.evidence_id.as_str())?;

		if evidence.quote.is_none() && evidence.selector.is_none() {
			return Err(eyre::eyre!(
				"{} required evidence {} must provide quote or selector.",
				path.display(),
				evidence.evidence_id
			));
		}

		if let Some(quote) = &evidence.quote
			&& let Some(text) = corpus_text.get(evidence.evidence_id.as_str())
			&& !text.contains(quote)
		{
			return Err(eyre::eyre!(
				"{} required evidence quote for {} is not present in corpus text.",
				path.display(),
				evidence.evidence_id
			));
		}
	}
	for (claim_id, link) in &job.expected_answer.evidence_links {
		if claim_id.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty evidence link claim id.", path.display()));
		}

		for evidence_id in link.ids() {
			ensure_known_evidence(path, &evidence_ids, evidence_id.as_str())?;
		}
	}

	Ok(())
}

fn validate_consolidation_fixture(job: &RealWorldJob, path: &Path) -> Result<()> {
	let consolidation =
		job.corpus.adapter_response.as_ref().and_then(|response| response.consolidation.as_ref());

	if job.suite == "consolidation" && consolidation.is_none() {
		return Err(eyre::eyre!(
			"{} consolidation jobs must provide adapter_response.consolidation.",
			path.display()
		));
	}

	let Some(consolidation) = consolidation else {
		return Ok(());
	};

	if consolidation.proposals.is_empty() && consolidation.executable_gaps.is_empty() {
		return Err(eyre::eyre!(
			"{} consolidation fixture must provide proposals or executable_gaps.",
			path.display()
		));
	}

	for proposal in &consolidation.proposals {
		validate_consolidation_proposal(proposal, path)?;
	}
	for gap in &consolidation.executable_gaps {
		if gap.primitive.trim().is_empty()
			|| gap.follow_up_issue.trim().is_empty()
			|| gap.reason.trim().is_empty()
		{
			return Err(eyre::eyre!(
				"{} has an incomplete consolidation executable gap.",
				path.display()
			));
		}
	}

	Ok(())
}

fn validate_consolidation_proposal(
	proposal: &ConsolidationProposalFixture,
	path: &Path,
) -> Result<()> {
	if proposal.proposal_id.trim().is_empty()
		|| proposal.proposal_kind.trim().is_empty()
		|| proposal.source_refs.is_empty()
		|| proposal.expected_source_refs.is_empty()
	{
		return Err(eyre::eyre!(
			"{} has an incomplete consolidation proposal fixture.",
			path.display()
		));
	}
	if !proposal.usefulness_score.is_finite()
		|| !proposal.min_usefulness_score.is_finite()
		|| !(0.0..=1.0).contains(&proposal.usefulness_score)
		|| !(0.0..=1.0).contains(&proposal.min_usefulness_score)
	{
		return Err(eyre::eyre!(
			"{} has invalid consolidation proposal usefulness scores.",
			path.display()
		));
	}
	if !proposal.diff.is_null() && !proposal.diff.is_object() {
		return Err(eyre::eyre!(
			"{} consolidation proposal diff must be a JSON object when present.",
			path.display()
		));
	}
	if proposal.unsupported_claim_flags.iter().any(|flag| !flag.is_object()) {
		return Err(eyre::eyre!(
			"{} consolidation unsupported-claim flags must be JSON objects.",
			path.display()
		));
	}

	Ok(())
}

fn validate_adapter_response(job: &RealWorldJob, path: &Path) -> Result<()> {
	let Some(adapter_response) = &job.corpus.adapter_response else {
		return Ok(());
	};
	let evidence_ids = corpus_evidence_ids(job);
	let event_ids = timeline_event_ids(job);

	for page in &adapter_response.answer.pages {
		validate_page_artifact(page, path, &evidence_ids, &event_ids)?;
	}

	Ok(())
}

fn validate_page_artifact(
	page: &DerivedPageArtifact,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
	event_ids: &BTreeSet<String>,
) -> Result<()> {
	if page.page_id.trim().is_empty()
		|| page.page_type.trim().is_empty()
		|| page.title.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete derived page.", path.display()));
	}

	for section in &page.sections {
		if section.section_id.trim().is_empty()
			|| section.heading.trim().is_empty()
			|| section.role.trim().is_empty()
			|| section.content.trim().is_empty()
		{
			return Err(eyre::eyre!(
				"{} page {} has an incomplete section.",
				path.display(),
				page.page_id
			));
		}

		for evidence_id in &section.evidence_ids {
			ensure_known_evidence(path, evidence_ids, evidence_id)?;
		}
		for event_id in &section.timeline_event_ids {
			ensure_known_event(path, event_ids, event_id)?;
		}
	}
	for backlink in &page.backlinks {
		if backlink.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} page {} has an empty backlink.",
				path.display(),
				page.page_id
			));
		}
	}
	for finding in &page.lint_findings {
		if finding.finding_id.trim().is_empty()
			|| finding.finding_type.trim().is_empty()
			|| finding.severity.trim().is_empty()
			|| finding.text.trim().is_empty()
		{
			return Err(eyre::eyre!(
				"{} page {} has an incomplete lint finding.",
				path.display(),
				page.page_id
			));
		}

		for evidence_id in &finding.evidence_ids {
			ensure_known_evidence(path, evidence_ids, evidence_id)?;
		}
	}

	if let Some(rebuild) = &page.rebuild
		&& (rebuild.first_hash.trim().is_empty() || rebuild.second_hash.trim().is_empty())
	{
		return Err(eyre::eyre!(
			"{} page {} has an incomplete rebuild record.",
			path.display(),
			page.page_id
		));
	}

	Ok(())
}

fn validate_scoring_rubric(job: &RealWorldJob, path: &Path) -> Result<()> {
	if !(0.0..=1.0).contains(&job.scoring_rubric.pass_threshold) {
		return Err(eyre::eyre!("{} has invalid pass_threshold.", path.display()));
	}
	if job.scoring_rubric.dimensions.is_empty() {
		return Err(eyre::eyre!("{} has no scoring dimensions.", path.display()));
	}

	for (dimension_id, dimension) in &job.scoring_rubric.dimensions {
		if dimension_id.trim().is_empty()
			|| !dimension.weight.is_finite()
			|| !dimension.max_points.is_finite()
			|| dimension.weight <= 0.0
			|| dimension.max_points <= 0.0
			|| dimension.criteria.is_null()
		{
			return Err(eyre::eyre!(
				"{} has invalid scoring dimension {}.",
				path.display(),
				dimension_id
			));
		}
	}
	for rule in &job.scoring_rubric.hard_fail_rules {
		if rule.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty hard fail rule.", path.display()));
		}
	}

	Ok(())
}

fn validate_allowed_uncertainty(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.allowed_uncertainty.fallback_action.trim().is_empty() {
		return Err(eyre::eyre!("{} has an empty fallback action.", path.display()));
	}
	if job.allowed_uncertainty.can_answer_unknown
		&& job.allowed_uncertainty.acceptable_phrases.is_empty()
	{
		return Err(eyre::eyre!(
			"{} allows unknown answers but defines no acceptable uncertainty phrase.",
			path.display()
		));
	}

	for phrase in &job.allowed_uncertainty.acceptable_phrases {
		if phrase.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty uncertainty phrase.", path.display()));
		}
	}

	Ok(())
}

fn validate_operator_debug(job: &RealWorldJob, path: &Path) -> Result<()> {
	let Some(debug) = &job.operator_debug else {
		if job.suite == "operator_debugging_ux" {
			return Err(eyre::eyre!(
				"{} operator_debugging_ux job must include operator_debug.",
				path.display()
			));
		}

		return Ok(());
	};

	if debug.failure_mode.trim().is_empty()
		|| debug.root_cause.trim().is_empty()
		|| debug.dropped_candidate_visibility.trim().is_empty()
		|| debug.trace_completeness.trim().is_empty()
		|| debug.repair_action_clarity.trim().is_empty()
		|| debug.steps_to_root_cause == 0
	{
		return Err(eyre::eyre!("{} has incomplete operator_debug evidence.", path.display()));
	}

	validate_optional_debug_field(path, debug.trace_id.as_deref(), "trace_id")?;
	validate_optional_debug_field(path, debug.viewer_url.as_deref(), "viewer_url")?;
	validate_optional_debug_field(
		path,
		debug.admin_trace_bundle_url.as_deref(),
		"admin_trace_bundle_url",
	)?;
	validate_optional_debug_field(path, debug.replay_command.as_deref(), "replay_command")?;
	validate_optional_debug_field(path, debug.replay_artifact.as_deref(), "replay_artifact")?;
	validate_non_empty_debug_list(path, &debug.viewer_panels, "viewer_panels")?;
	validate_non_empty_debug_list(path, &debug.cli_steps, "cli_steps")?;
	validate_non_empty_debug_list(path, &debug.trace_evidence, "trace_evidence")?;

	for gap in &debug.ux_gaps {
		if gap.gap_id.trim().is_empty()
			|| gap.severity.trim().is_empty()
			|| gap.description.trim().is_empty()
			|| gap.follow_up_issue.trim().is_empty()
		{
			return Err(eyre::eyre!("{} has incomplete operator_debug ux_gaps.", path.display()));
		}
	}

	Ok(())
}

fn validate_job_encoding(job: &RealWorldJob, path: &Path) -> Result<()> {
	if let Some(status) = job.encoding.status {
		if !matches!(
			status,
			TypedStatus::NotEncoded | TypedStatus::Blocked | TypedStatus::Incomplete
		) {
			return Err(eyre::eyre!(
				"{} job {} uses encoding.status {}; only not_encoded, blocked, or incomplete are allowed.",
				path.display(),
				job.job_id,
				status_str(status)
			));
		}
		if job.encoding.reason.as_deref().is_none_or(|reason| reason.trim().is_empty()) {
			return Err(eyre::eyre!(
				"{} job {} declares encoding.status but no reason.",
				path.display(),
				job.job_id
			));
		}
	}
	if let Some(follow_up) = &job.encoding.follow_up
		&& (follow_up.title.trim().is_empty() || follow_up.reason.trim().is_empty())
	{
		return Err(eyre::eyre!(
			"{} job {} has an incomplete encoding follow-up.",
			path.display(),
			job.job_id
		));
	}

	Ok(())
}

fn validate_memory_evolution(job: &RealWorldJob, path: &Path) -> Result<()> {
	let Some(evolution) = &job.memory_evolution else {
		return Ok(());
	};
	let evidence_ids = corpus_evidence_ids(job);
	let trap_ids =
		job.negative_traps.iter().map(|trap| trap.trap_id.as_str()).collect::<BTreeSet<_>>();

	for evidence_id in
		evolution.current_evidence_ids.iter().chain(evolution.historical_evidence_ids.iter())
	{
		ensure_known_evidence(path, &evidence_ids, evidence_id)?;
	}
	for trap_id in &evolution.stale_trap_ids {
		if !trap_ids.contains(trap_id.as_str()) {
			return Err(eyre::eyre!(
				"{} job {} references unknown stale trap id {}.",
				path.display(),
				job.job_id,
				trap_id
			));
		}
	}
	for conflict in &evolution.conflicts {
		validate_evolution_conflict(path, &evidence_ids, conflict)?;
	}

	if let Some(rationale) = &evolution.update_rationale {
		validate_update_rationale(path, &evidence_ids, rationale)?;
	}
	if let Some(temporal) = &evolution.temporal_validity {
		validate_temporal_validity(job, path, temporal)?;
	}

	Ok(())
}

fn validate_evolution_conflict(
	path: &Path,
	evidence_ids: &BTreeSet<String>,
	conflict: &EvolutionConflict,
) -> Result<()> {
	if conflict.conflict_id.trim().is_empty() || conflict.claim_id.trim().is_empty() {
		return Err(eyre::eyre!("{} has an incomplete evolution conflict.", path.display()));
	}

	ensure_known_evidence(path, evidence_ids, conflict.current_evidence_id.as_str())?;
	ensure_known_evidence(path, evidence_ids, conflict.historical_evidence_id.as_str())?;

	if let Some(evidence_id) = &conflict.resolved_by_evidence_id {
		ensure_known_evidence(path, evidence_ids, evidence_id)?;
	}

	Ok(())
}

fn validate_update_rationale(
	path: &Path,
	evidence_ids: &BTreeSet<String>,
	rationale: &UpdateRationale,
) -> Result<()> {
	if rationale.claim_id.trim().is_empty() {
		return Err(eyre::eyre!(
			"{} has an update rationale with an empty claim_id.",
			path.display()
		));
	}

	for evidence_id in &rationale.evidence_ids {
		ensure_known_evidence(path, evidence_ids, evidence_id)?;
	}

	Ok(())
}

fn validate_temporal_validity(
	job: &RealWorldJob,
	path: &Path,
	temporal: &TemporalValidity,
) -> Result<()> {
	if temporal.follow_up.as_deref().is_some_and(|follow_up| follow_up.trim().is_empty()) {
		return Err(eyre::eyre!(
			"{} job {} has an empty temporal validity follow-up.",
			path.display(),
			job.job_id
		));
	}
	if temporal.required
		&& !temporal.encoded
		&& !matches!(job.encoding.status, Some(TypedStatus::NotEncoded | TypedStatus::Blocked))
	{
		return Err(eyre::eyre!(
			"{} job {} requires temporal validity but does not declare a not_encoded or blocked encoding status.",
			path.display(),
			job.job_id
		));
	}

	Ok(())
}

fn validate_trace_explainability(job: &RealWorldJob, path: &Path) -> Result<()> {
	let Some(trace) = job
		.corpus
		.adapter_response
		.as_ref()
		.and_then(|response| response.answer.trace_explainability.as_ref())
	else {
		return Ok(());
	};
	let known = corpus_evidence_ids(job);
	let stage_names =
		trace.stages.iter().map(|stage| stage.stage_name.as_str()).collect::<BTreeSet<_>>();

	if trace.trace_id.as_deref().is_some_and(str::is_empty) {
		return Err(eyre::eyre!("{} has an empty trace_explainability trace_id.", path.display()));
	}
	if trace.failure_stage.as_deref().is_some_and(str::is_empty) {
		return Err(eyre::eyre!(
			"{} has an empty trace_explainability failure_stage.",
			path.display()
		));
	}

	if let Some(failure_stage) = trace.failure_stage.as_deref()
		&& !stage_names.is_empty()
		&& !stage_names.contains(failure_stage)
	{
		return Err(eyre::eyre!(
			"{} trace_explainability failure_stage {} is not present in stages.",
			path.display(),
			failure_stage
		));
	}

	for stage in &trace.stages {
		validate_trace_stage(stage, &known, path)?;
	}

	Ok(())
}

fn validate_optional_debug_field(path: &Path, value: Option<&str>, field: &str) -> Result<()> {
	if value.is_some_and(|value| value.trim().is_empty()) {
		return Err(eyre::eyre!("{} has empty operator_debug {field}.", path.display()));
	}

	Ok(())
}

fn validate_non_empty_debug_list(path: &Path, values: &[String], field: &str) -> Result<()> {
	if values.iter().any(|value| value.trim().is_empty()) {
		return Err(eyre::eyre!("{} has empty operator_debug {field} entry.", path.display()));
	}

	Ok(())
}

fn validate_trace_stage(
	stage: &TraceStageExplainability,
	known: &BTreeSet<String>,
	path: &Path,
) -> Result<()> {
	if stage.stage_name.trim().is_empty() {
		return Err(eyre::eyre!("{} has a trace stage with an empty stage_name.", path.display()));
	}

	for evidence_id in stage
		.kept_evidence
		.iter()
		.chain(stage.dropped_evidence.iter())
		.chain(stage.demoted_evidence.iter())
		.chain(stage.distractor_evidence.iter())
	{
		ensure_known_evidence(path, known, evidence_id)?;
	}

	Ok(())
}

fn validate_required_rfc3339(value: &str, path: &Path, id: &str) -> Result<()> {
	if OffsetDateTime::parse(value, &Rfc3339).is_err() {
		return Err(eyre::eyre!("{} has invalid RFC3339 timestamp for {}.", path.display(), id));
	}

	Ok(())
}

fn validate_optional_rfc3339(value: &str, path: &Path, id: &str) -> Result<()> {
	if !value.trim().is_empty() {
		validate_required_rfc3339(value, path, id)?;
	}

	Ok(())
}

fn ensure_known_evidence(path: &Path, known: &BTreeSet<String>, evidence_id: &str) -> Result<()> {
	if !known.contains(evidence_id) {
		return Err(eyre::eyre!(
			"{} references unknown evidence id {}.",
			path.display(),
			evidence_id
		));
	}

	Ok(())
}

fn corpus_evidence_ids(job: &RealWorldJob) -> BTreeSet<String> {
	job.corpus.items.iter().map(|item| item.evidence_id.clone()).collect()
}

fn corpus_text_by_id(job: &RealWorldJob) -> BTreeMap<&str, &str> {
	job.corpus
		.items
		.iter()
		.filter_map(|item| item.text.as_deref().map(|text| (item.evidence_id.as_str(), text)))
		.collect()
}

fn timeline_event_ids(job: &RealWorldJob) -> BTreeSet<String> {
	job.timeline.iter().map(|event| event.event_id.clone()).collect()
}

fn ensure_known_event(path: &Path, known: &BTreeSet<String>, event_id: &str) -> Result<()> {
	if !known.contains(event_id) {
		return Err(eyre::eyre!(
			"{} references unknown timeline event id {}.",
			path.display(),
			event_id
		));
	}

	Ok(())
}

fn build_report(jobs: &[RealWorldJob], args: &RunArgs) -> Result<RealWorldReport> {
	if jobs.is_empty() {
		return Err(eyre::eyre!("At least one real_world_job fixture is required."));
	}

	let mut job_reports = Vec::with_capacity(jobs.len());
	let mut unsupported_claims = Vec::new();

	for job in jobs {
		let scoring = score_job(job);

		unsupported_claims.extend(scoring.unsupported_claims.clone());
		job_reports.push(job_report(job, scoring));
	}

	let suites = suite_reports(&job_reports);
	let not_encoded_suites = suites
		.iter()
		.filter(|suite| suite.status == TypedStatus::NotEncoded)
		.map(|suite| suite.suite_id.clone())
		.collect::<Vec<_>>();
	let summary = report_summary(&job_reports, &suites);
	let evolution = evolution_summary(&job_reports);
	let follow_ups = follow_up_reports(jobs);
	let external_adapters = external_adapter_section(
		&args.external_adapter_manifest,
		args.skip_external_adapter_manifest,
	)?;

	Ok(RealWorldReport {
		schema: REPORT_SCHEMA.to_string(),
		run_id: args.run_id.clone(),
		generated_at: OffsetDateTime::now_utc().format(&Rfc3339)?,
		runner_version: VERSION.to_string(),
		corpus_profile: corpus_profile(jobs),
		adapter: adapter_report(args)?,
		external_adapters,
		capture_integration: capture_integration_report(jobs),
		summary,
		suites,
		jobs: job_reports,
		unsupported_claims,
		not_encoded_suites,
		private_corpus_redaction: private_corpus_redaction(jobs),
		evolution,
		follow_ups,
	})
}

fn score_job(job: &RealWorldJob) -> JobScoring {
	let answer = produced_answer(job);
	let produced_evidence = produced_evidence_ids(answer);
	let trap_ids_used = trap_ids_used(job, &produced_evidence);
	let consolidation = consolidation_job_report(job);

	if let Some(status) = job.encoding.status {
		let evolution = evolution_job_report(job, answer, &trap_ids_used, 0);

		return JobScoring {
			status,
			normalized_score: 0.0,
			hard_fail_hits: Vec::new(),
			unsupported_claims: Vec::new(),
			wrong_result_count: 0,
			knowledge: None,
			trap_ids_used,
			dimension_scores: declared_not_encoded_dimension_scores(job),
			reason: job
				.encoding
				.reason
				.clone()
				.unwrap_or_else(|| "Job did not reach a runnable scoring state.".to_string()),
			evolution,
			consolidation,
		};
	}

	let missing_claims = missing_required_claims(job, answer);
	let forbidden_claims = forbidden_claim_hits(job, answer);
	let missing_evidence = missing_required_evidence(job, &produced_evidence);
	let knowledge = knowledge_metrics(job, answer);
	let mut unsupported_claims = unsupported_claims(job, answer);

	unsupported_claims.extend(unsupported_page_claims(answer));

	let operator_counts = operator_debug_failure_counts(job);
	let latency_violations = latency_violations(job, answer);
	let hard_fail_hits = hard_fail_hits(job, &unsupported_claims, &trap_ids_used);
	let evolution = evolution_job_report(job, answer, &trap_ids_used, forbidden_claims.len());
	let stale_answers = evolution.as_ref().map_or(0, |report| report.stale_answer_count);
	let conflict_detection_missing = evolution
		.as_ref()
		.map_or(0, |report| report.conflict_count - report.conflict_detection_count);
	let update_rationale_missing = evolution.as_ref().map_or(0, update_rationale_missing_count);
	let counts = FailureCounts {
		missing_claims: missing_claims.len(),
		forbidden_claims: forbidden_claims.len(),
		missing_evidence: missing_evidence.len(),
		trap_uses: trap_ids_used.len(),
		unsupported_claims: unsupported_claims.len(),
		operator_debug_missing: operator_counts.operator_debug_missing,
		operator_debug_raw_sql: operator_counts.operator_debug_raw_sql,
		operator_debug_trace_gaps: operator_counts.operator_debug_trace_gaps,
		operator_debug_repair_unclear: operator_counts.operator_debug_repair_unclear,
		stale_answers,
		conflict_detection_missing,
		update_rationale_missing,
		latency_violations,
		proposal_usefulness_failures: proposal_usefulness_failures(consolidation.as_ref()),
		lineage_failures: lineage_failures(consolidation.as_ref()),
		review_action_failures: review_action_failures(consolidation.as_ref()),
		source_mutations: consolidation.as_ref().map_or(0, |report| report.source_mutation_count),
		blocking_executable_gaps: blocking_executable_gaps(consolidation.as_ref()),
		untraced_page_sections: knowledge
			.as_ref()
			.map_or(0, |metrics| metrics.untraced_section_count),
		missed_stale_findings: knowledge.as_ref().map_or(0, missed_stale_finding_count),
		rebuild_failures: knowledge.as_ref().map_or(0, |metrics| metrics.rebuild_failure_count),
		page_usefulness_failures: knowledge.as_ref().map_or(0, page_usefulness_failure_count),
	};
	let dimension_scores = dimension_scores(job, &counts);
	let normalized_score = normalized_score(&dimension_scores);
	let wrong_result_count = counts.missing_claims
		+ counts.forbidden_claims
		+ counts.missing_evidence
		+ counts.trap_uses
		+ counts.operator_debug_missing
		+ counts.operator_debug_raw_sql
		+ counts.operator_debug_trace_gaps
		+ counts.operator_debug_repair_unclear
		+ counts.conflict_detection_missing
		+ counts.update_rationale_missing
		+ counts.proposal_usefulness_failures
		+ counts.lineage_failures
		+ counts.review_action_failures
		+ counts.untraced_page_sections
		+ counts.missed_stale_findings
		+ counts.rebuild_failures
		+ counts.page_usefulness_failures;
	let status = job_status(
		normalized_score,
		job.scoring_rubric.pass_threshold,
		wrong_result_count,
		unsupported_claims.len(),
		counts.source_mutations,
		counts.blocking_executable_gaps,
	);
	let reason = job_reason(status, &counts, normalized_score);

	for claim in &mut unsupported_claims {
		claim.suite_id = job.suite.clone();
		claim.job_id = job.job_id.clone();
	}

	JobScoring {
		status,
		normalized_score,
		hard_fail_hits,
		unsupported_claims,
		wrong_result_count,
		knowledge,
		trap_ids_used,
		dimension_scores,
		reason,
		evolution,
		consolidation,
	}
}

fn operator_debug_failure_counts(job: &RealWorldJob) -> FailureCounts {
	let Some(debug) = &job.operator_debug else {
		return FailureCounts {
			operator_debug_missing: usize::from(job.suite == "operator_debugging_ux"),
			..FailureCounts::default()
		};
	};

	FailureCounts {
		operator_debug_raw_sql: usize::from(debug.raw_sql_needed),
		operator_debug_trace_gaps: usize::from(debug.trace_completeness != "complete"),
		operator_debug_repair_unclear: usize::from(debug.repair_action_clarity != "clear"),
		..FailureCounts::default()
	}
}

fn declared_not_encoded_dimension_scores(job: &RealWorldJob) -> Vec<DimensionScoreReport> {
	job.scoring_rubric
		.dimensions
		.iter()
		.map(|(dimension_id, dimension)| DimensionScoreReport {
			dimension: dimension_id.clone(),
			score: 0.0,
			max_points: dimension.max_points,
			weight: dimension.weight,
		})
		.collect()
}

fn produced_answer(job: &RealWorldJob) -> &ProducedAnswer {
	job.corpus
		.adapter_response
		.as_ref()
		.map(|response| &response.answer)
		.unwrap_or_else(|| synthetic_answer(job))
}

fn synthetic_answer(job: &RealWorldJob) -> &ProducedAnswer {
	let _ = job;

	static EMPTY_ANSWER: std::sync::OnceLock<ProducedAnswer> = std::sync::OnceLock::new();

	EMPTY_ANSWER.get_or_init(|| ProducedAnswer {
		content: String::new(),
		claims: Vec::new(),
		evidence_ids: Vec::new(),
		pages: Vec::new(),
		latency_ms: None,
		cost: None,
		trace_explainability: None,
	})
}

fn produced_evidence_ids(answer: &ProducedAnswer) -> BTreeSet<String> {
	let mut evidence = answer.evidence_ids.iter().cloned().collect::<BTreeSet<_>>();

	for claim in &answer.claims {
		evidence.extend(claim.evidence_ids.iter().cloned());
	}

	evidence
}

fn missing_required_claims(job: &RealWorldJob, answer: &ProducedAnswer) -> Vec<String> {
	job.expected_answer
		.must_include
		.iter()
		.filter(|claim| !claim_is_present(claim, answer))
		.map(|claim| claim.text().to_string())
		.collect()
}

fn claim_is_present(claim: &ExpectedClaim, answer: &ProducedAnswer) -> bool {
	if let Some(claim_id) = claim.claim_id()
		&& answer.claims.iter().any(|produced| produced.claim_id.as_deref() == Some(claim_id))
	{
		return true;
	}

	answer.content.contains(claim.text())
}

fn forbidden_claim_hits(job: &RealWorldJob, answer: &ProducedAnswer) -> Vec<String> {
	job.expected_answer
		.must_not_include
		.iter()
		.filter(|claim| answer.content.contains(claim.as_str()))
		.cloned()
		.collect()
}

fn missing_required_evidence(
	job: &RealWorldJob,
	produced_evidence: &BTreeSet<String>,
) -> Vec<String> {
	job.required_evidence
		.iter()
		.filter(|evidence| {
			is_required_use(evidence) && !produced_evidence.contains(&evidence.evidence_id)
		})
		.map(|evidence| evidence.evidence_id.clone())
		.collect()
}

fn is_required_use(evidence: &RequiredEvidence) -> bool {
	matches!(evidence.requirement.as_str(), "cite" | "use" | "explain")
}

fn trap_ids_used(job: &RealWorldJob, produced_evidence: &BTreeSet<String>) -> Vec<String> {
	job.negative_traps
		.iter()
		.filter(|trap| trap.failure_if_used)
		.filter(|trap| {
			trap.evidence_ids.iter().any(|evidence_id| produced_evidence.contains(evidence_id))
		})
		.map(|trap| trap.trap_id.clone())
		.collect()
}

fn evolution_job_report(
	job: &RealWorldJob,
	answer: &ProducedAnswer,
	trap_ids_used: &[String],
	forbidden_claim_count: usize,
) -> Option<EvolutionJobReport> {
	let evolution = job.memory_evolution.as_ref()?;
	let stale_trap_ids_used = stale_trap_ids_used(job, evolution, trap_ids_used);
	let stale_answer_count =
		stale_answer_count(job, evolution, &stale_trap_ids_used, forbidden_claim_count);
	let conflict_detection_count = evolution
		.conflicts
		.iter()
		.filter(|conflict| conflict_is_detected(conflict, answer))
		.count();
	let update_rationale_available = evolution
		.update_rationale
		.as_ref()
		.is_some_and(|rationale| update_rationale_is_available(rationale, answer));
	let temporal_validity_required =
		evolution.temporal_validity.as_ref().is_some_and(|temporal| temporal.required);
	let temporal_validity_encoded =
		evolution.temporal_validity.as_ref().is_some_and(|temporal| temporal.encoded);
	let temporal_validity_not_encoded = temporal_validity_required && !temporal_validity_encoded;
	let history_readback_encoded =
		evolution.history_readback.as_ref().is_some_and(|history| history.encoded);
	let history_event_types = evolution
		.history_readback
		.as_ref()
		.map_or_else(Vec::new, |history| history.required_event_types.clone());
	let history_requires_note_version_links = evolution
		.history_readback
		.as_ref()
		.is_some_and(|history| history.requires_note_version_links);
	let follow_up = evolution
		.temporal_validity
		.as_ref()
		.and_then(|temporal| temporal.follow_up.clone())
		.or_else(|| job.encoding.follow_up.as_ref().map(|follow_up| follow_up.title.clone()));

	Some(EvolutionJobReport {
		current_evidence: evolution.current_evidence_ids.clone(),
		historical_evidence: evolution.historical_evidence_ids.clone(),
		stale_answer_count,
		stale_trap_ids_used,
		conflict_count: evolution.conflicts.len(),
		conflict_detection_count,
		update_rationale_available,
		temporal_validity_required,
		temporal_validity_encoded,
		temporal_validity_not_encoded,
		history_readback_encoded,
		history_event_types,
		history_requires_note_version_links,
		follow_up,
	})
}

fn stale_answer_count(
	job: &RealWorldJob,
	evolution: &MemoryEvolution,
	stale_trap_ids_used: &[String],
	forbidden_claim_count: usize,
) -> usize {
	let stale_trap_count = if evolution.stale_trap_ids.is_empty() {
		job.negative_traps.iter().filter(|trap| trap.trap_type == "stale_fact").count()
	} else {
		evolution.stale_trap_ids.len()
	};
	let stale_forbidden_claims = if stale_trap_count > 0 { forbidden_claim_count } else { 0 };

	stale_trap_ids_used.len().max(stale_forbidden_claims)
}

fn stale_trap_ids_used(
	job: &RealWorldJob,
	evolution: &MemoryEvolution,
	trap_ids_used: &[String],
) -> Vec<String> {
	let declared_stale_traps = if evolution.stale_trap_ids.is_empty() {
		job.negative_traps
			.iter()
			.filter(|trap| trap.trap_type == "stale_fact")
			.map(|trap| trap.trap_id.as_str())
			.collect::<BTreeSet<_>>()
	} else {
		evolution.stale_trap_ids.iter().map(String::as_str).collect::<BTreeSet<_>>()
	};

	trap_ids_used
		.iter()
		.filter(|trap_id| declared_stale_traps.contains(trap_id.as_str()))
		.cloned()
		.collect()
}

fn conflict_is_detected(conflict: &EvolutionConflict, answer: &ProducedAnswer) -> bool {
	let mut required_evidence =
		vec![conflict.current_evidence_id.as_str(), conflict.historical_evidence_id.as_str()];

	if let Some(evidence_id) = &conflict.resolved_by_evidence_id {
		required_evidence.push(evidence_id.as_str());
	}

	answer.claims.iter().any(|claim| {
		claim.claim_id.as_deref() == Some(conflict.claim_id.as_str())
			&& required_evidence
				.iter()
				.all(|evidence_id| claim.evidence_ids.iter().any(|id| id == evidence_id))
	})
}

fn update_rationale_is_available(rationale: &UpdateRationale, answer: &ProducedAnswer) -> bool {
	if !rationale.available {
		return false;
	}

	answer.claims.iter().any(|claim| {
		claim.claim_id.as_deref() == Some(rationale.claim_id.as_str())
			&& !claim.evidence_ids.is_empty()
			&& rationale.evidence_ids.iter().any(|evidence_id| {
				claim.evidence_ids.iter().any(|produced| produced == evidence_id)
			})
	})
}

fn update_rationale_missing_count(report: &EvolutionJobReport) -> usize {
	if report.update_rationale_available || report.temporal_validity_not_encoded {
		0
	} else if report.conflict_count > 0 {
		1
	} else {
		0
	}
}

fn unsupported_claims(job: &RealWorldJob, answer: &ProducedAnswer) -> Vec<UnsupportedClaimReport> {
	answer.claims.iter().filter_map(|claim| unsupported_claim(job, claim)).collect()
}

fn unsupported_claim(job: &RealWorldJob, claim: &ProducedClaim) -> Option<UnsupportedClaimReport> {
	let Some(claim_id) = claim.claim_id.as_deref() else {
		return Some(unsupported_claim_report(claim, "claim has no claim_id"));
	};
	let Some(allowed) = job.expected_answer.evidence_links.get(claim_id).map(EvidenceLink::ids)
	else {
		return Some(unsupported_claim_report(
			claim,
			"claim_id is not present in expected_answer.evidence_links",
		));
	};

	if claim.evidence_ids.is_empty() {
		return Some(unsupported_claim_report(claim, "claim has no produced evidence ids"));
	}
	if !claim.evidence_ids.iter().any(|evidence_id| allowed.contains(evidence_id)) {
		return Some(unsupported_claim_report(
			claim,
			"claim evidence is not allowed for this claim_id",
		));
	}

	None
}

fn unsupported_claim_report(claim: &ProducedClaim, reason: &str) -> UnsupportedClaimReport {
	UnsupportedClaimReport {
		suite_id: String::new(),
		job_id: String::new(),
		claim_id: claim.claim_id.clone(),
		claim_text: bounded_text(claim.text.as_str(), 240),
		reason: reason.to_string(),
		evidence_ids: claim.evidence_ids.clone(),
	}
}

fn unsupported_page_claims(answer: &ProducedAnswer) -> Vec<UnsupportedClaimReport> {
	answer
		.pages
		.iter()
		.flat_map(|page| {
			page.sections.iter().filter_map(|section| {
				if section_is_traced(section) || section_is_flagged_unsupported(section) {
					return None;
				}

				Some(UnsupportedClaimReport {
					suite_id: String::new(),
					job_id: String::new(),
					claim_id: Some(format!("{}:{}", page.page_id, section.section_id)),
					claim_text: bounded_text(section.content.as_str(), 240),
					reason:
						"derived page section has no source evidence and is not flagged unsupported"
							.to_string(),
					evidence_ids: section.evidence_ids.clone(),
				})
			})
		})
		.collect()
}

fn knowledge_metrics(job: &RealWorldJob, answer: &ProducedAnswer) -> Option<KnowledgeJobMetrics> {
	if answer.pages.is_empty() {
		return None;
	}

	let mut metrics = KnowledgeJobMetrics {
		page_count: answer.pages.len(),
		stale_trap_count: stale_traps(job).len(),
		..KnowledgeJobMetrics::default()
	};

	for page in &answer.pages {
		accumulate_page_metrics(page, &mut metrics);
	}

	metrics.stale_traps_detected = stale_traps(job)
		.iter()
		.filter(|trap| page_artifacts_detect_stale_trap(&answer.pages, trap))
		.count();
	metrics.citation_coverage = ratio(metrics.traced_section_count, metrics.section_count);
	metrics.stale_claim_detection =
		ratio_or_full(metrics.stale_traps_detected, metrics.stale_trap_count);
	metrics.rebuild_determinism = ratio(metrics.deterministic_rebuild_count, metrics.page_count);
	metrics.backlink_coverage = ratio(metrics.pages_with_backlinks, metrics.page_count);
	metrics.page_usefulness = round3(
		(metrics.citation_coverage
			+ metrics.stale_claim_detection
			+ metrics.rebuild_determinism
			+ metrics.backlink_coverage)
			/ 4.0,
	);

	Some(metrics)
}

fn stale_traps(job: &RealWorldJob) -> Vec<&NegativeTrap> {
	job.negative_traps
		.iter()
		.filter(|trap| trap.trap_type == "stale_fact" && trap.failure_if_used)
		.collect()
}

fn accumulate_page_metrics(page: &DerivedPageArtifact, metrics: &mut KnowledgeJobMetrics) {
	if !page.backlinks.is_empty() {
		metrics.pages_with_backlinks += 1;
	}

	metrics.backlink_count += page.backlinks.len();

	for section in &page.sections {
		metrics.section_count += 1;

		if section_is_traced(section) {
			metrics.traced_section_count += 1;
		} else if section_is_flagged_unsupported(section) {
			metrics.flagged_unsupported_section_count += 1;

			if section.role == "summary" {
				metrics.unsupported_summary_count += 1;
			}
		} else {
			metrics.untraced_section_count += 1;
		}
	}

	if let Some(rebuild) = &page.rebuild {
		if !rebuild.allowed_variance.is_empty() {
			metrics.allowed_variance_count += 1;
		}
		if rebuild_is_acceptable(rebuild) {
			metrics.deterministic_rebuild_count += 1;
		} else {
			metrics.rebuild_failure_count += 1;
		}
	} else {
		metrics.rebuild_failure_count += 1;
	}

	metrics.rebuild_page_count += 1;
}

fn section_is_traced(section: &DerivedPageSection) -> bool {
	!section.evidence_ids.is_empty() || !section.timeline_event_ids.is_empty()
}

fn section_is_flagged_unsupported(section: &DerivedPageSection) -> bool {
	section.unsupported_reason.as_ref().is_some_and(|reason| !reason.trim().is_empty())
}

fn rebuild_is_acceptable(rebuild: &DerivedPageRebuild) -> bool {
	(rebuild.deterministic && rebuild.first_hash == rebuild.second_hash)
		|| !rebuild.allowed_variance.is_empty()
}

fn page_artifacts_detect_stale_trap(pages: &[DerivedPageArtifact], trap: &NegativeTrap) -> bool {
	pages.iter().any(|page| {
		page.lint_findings.iter().any(|finding| {
			finding.trap_id.as_deref() == Some(trap.trap_id.as_str())
				|| finding
					.evidence_ids
					.iter()
					.any(|evidence_id| trap.evidence_ids.contains(evidence_id))
		})
	})
}

fn missed_stale_finding_count(metrics: &KnowledgeJobMetrics) -> usize {
	metrics.stale_trap_count.saturating_sub(metrics.stale_traps_detected)
}

fn page_usefulness_failure_count(metrics: &KnowledgeJobMetrics) -> usize {
	if metrics.page_usefulness < 0.8 { 1 } else { 0 }
}

fn hard_fail_hits(
	job: &RealWorldJob,
	unsupported_claims: &[UnsupportedClaimReport],
	trap_ids_used: &[String],
) -> Vec<String> {
	let mut hits = Vec::new();

	if !unsupported_claims.is_empty() {
		hits.push(
			"unsupported high-confidence claim about a required decision or fact".to_string(),
		);
	}
	if !trap_ids_used.is_empty() {
		hits.push("use of a negative trap marked failure_if_used = true".to_string());
	}
	if job.expected_answer.requires_caveat && !answer_has_required_caveat(job, produced_answer(job))
	{
		hits.push("missing required caveat".to_string());
	}
	if job.expected_answer.requires_refusal && !answer_looks_like_refusal(produced_answer(job)) {
		hits.push("missing required refusal".to_string());
	}

	if let Some(consolidation) = consolidation_job_report(job) {
		if consolidation.source_mutation_count > 0 {
			hits.push(
				"source mutation count must remain zero for proposal-only consolidation cases"
					.to_string(),
			);
		}
		if consolidation.executable_gaps.iter().any(|gap| gap.blocks_fixture_pass) {
			hits.push(
				"missing consolidation primitive requires a precise follow-up issue".to_string(),
			);
		}
	}

	hits
}

fn answer_has_required_caveat(job: &RealWorldJob, answer: &ProducedAnswer) -> bool {
	job.allowed_uncertainty.acceptable_phrases.iter().any(|phrase| answer.content.contains(phrase))
}

fn answer_looks_like_refusal(answer: &ProducedAnswer) -> bool {
	let lower = answer.content.to_ascii_lowercase();

	lower.contains("cannot") || lower.contains("can't") || lower.contains("refuse")
}

fn dimension_scores(job: &RealWorldJob, counts: &FailureCounts) -> Vec<DimensionScoreReport> {
	job.scoring_rubric
		.dimensions
		.iter()
		.map(|(dimension_id, dimension)| DimensionScoreReport {
			dimension: dimension_id.clone(),
			score: dimension_score(dimension_id, dimension.max_points, counts),
			max_points: dimension.max_points,
			weight: dimension.weight,
		})
		.collect()
}

fn dimension_score(dimension_id: &str, max_points: f64, counts: &FailureCounts) -> f64 {
	let failed = match dimension_id {
		"answer_correctness" | "workflow_helpfulness" =>
			counts.missing_claims > 0
				|| counts.forbidden_claims > 0
				|| counts.operator_debug_repair_unclear > 0
				|| counts.conflict_detection_missing > 0
				|| counts.proposal_usefulness_failures > 0
				|| counts.review_action_failures > 0
				|| counts.page_usefulness_failures > 0,
		"evidence_grounding" =>
			counts.missing_evidence > 0
				|| counts.unsupported_claims > 0
				|| counts.lineage_failures > 0
				|| counts.untraced_page_sections > 0,
		"trap_avoidance" => counts.trap_uses > 0 || counts.missed_stale_findings > 0,
		"uncertainty_handling" => counts.unsupported_claims > 0,
		"lifecycle_behavior" =>
			counts.stale_answers > 0
				|| counts.conflict_detection_missing > 0
				|| counts.update_rationale_missing > 0
				|| counts.source_mutations > 0
				|| counts.rebuild_failures > 0,
		"source_immutability" => counts.source_mutations > 0,
		"proposal_usefulness" => counts.proposal_usefulness_failures > 0,
		"lineage_completeness" => counts.lineage_failures > 0,
		"review_action_correctness" => counts.review_action_failures > 0,
		"debuggability" =>
			counts.missing_claims > 0
				|| counts.unsupported_claims > 0
				|| counts.operator_debug_missing > 0
				|| counts.operator_debug_raw_sql > 0
				|| counts.operator_debug_trace_gaps > 0,
		"latency_resource" => counts.latency_violations > 0,
		"personalization_fit" | "ownership_correctness" =>
			counts.missing_claims > 0 || counts.unsupported_claims > 0,
		_ => counts.missing_claims > 0 || counts.unsupported_claims > 0 || counts.trap_uses > 0,
	};

	if failed { 0.0 } else { max_points }
}

fn latency_violations(job: &RealWorldJob, answer: &ProducedAnswer) -> usize {
	let Some(max_latency_ms) = latency_threshold_ms(job) else {
		return 0;
	};
	let Some(latency_ms) = answer.latency_ms else {
		return 1;
	};

	usize::from(latency_ms > max_latency_ms)
}

fn latency_threshold_ms(job: &RealWorldJob) -> Option<f64> {
	job.scoring_rubric
		.dimensions
		.get("latency_resource")
		.and_then(|dimension| dimension.criteria.get("max_latency_ms"))
		.and_then(Value::as_f64)
}

fn normalized_score(scores: &[DimensionScoreReport]) -> f64 {
	let total_weight = scores.iter().map(|score| score.weight).sum::<f64>();

	if total_weight == 0.0 {
		return 0.0;
	}

	scores.iter().map(|score| (score.score / score.max_points) * score.weight).sum::<f64>()
		/ total_weight
}

fn job_status(
	normalized_score: f64,
	pass_threshold: f64,
	wrong_result_count: usize,
	unsupported_claim_count: usize,
	source_mutation_count: usize,
	blocking_executable_gap_count: usize,
) -> TypedStatus {
	if unsupported_claim_count > 0 {
		TypedStatus::UnsupportedClaim
	} else if source_mutation_count > 0 {
		TypedStatus::LifecycleFail
	} else if blocking_executable_gap_count > 0 {
		TypedStatus::Blocked
	} else if wrong_result_count > 0 {
		TypedStatus::WrongResult
	} else if normalized_score >= pass_threshold {
		TypedStatus::Pass
	} else {
		TypedStatus::WrongResult
	}
}

fn job_reason(status: TypedStatus, counts: &FailureCounts, normalized_score: f64) -> String {
	let wrong_result_signal_count = wrong_result_signal_count(counts);

	match status {
		TypedStatus::Pass => format!("Job passed with normalized_score {normalized_score:.3}."),
		TypedStatus::UnsupportedClaim => format!(
			"Job produced {} unsupported claim(s), {} wrong-result signal(s), {} latency violation(s), and normalized_score {normalized_score:.3}.",
			counts.unsupported_claims, wrong_result_signal_count, counts.latency_violations
		),
		TypedStatus::WrongResult => format!(
			"Job produced {} wrong-result signal(s), {} latency violation(s), and normalized_score {normalized_score:.3}.",
			wrong_result_signal_count, counts.latency_violations
		),
		TypedStatus::LifecycleFail => format!(
			"Job produced {} source mutation(s) and normalized_score {normalized_score:.3}.",
			counts.source_mutations
		),
		TypedStatus::Blocked => format!(
			"Job has {} blocking executable gap(s) and normalized_score {normalized_score:.3}.",
			counts.blocking_executable_gaps
		),
		_ => "Job did not reach a runnable scoring state.".to_string(),
	}
}

fn wrong_result_signal_count(counts: &FailureCounts) -> usize {
	counts.missing_claims
		+ counts.forbidden_claims
		+ counts.missing_evidence
		+ counts.trap_uses
		+ counts.operator_debug_missing
		+ counts.operator_debug_raw_sql
		+ counts.operator_debug_trace_gaps
		+ counts.operator_debug_repair_unclear
		+ counts.conflict_detection_missing
		+ counts.update_rationale_missing
		+ counts.proposal_usefulness_failures
		+ counts.lineage_failures
		+ counts.review_action_failures
		+ counts.untraced_page_sections
		+ counts.missed_stale_findings
		+ counts.rebuild_failures
		+ counts.page_usefulness_failures
}

fn job_report(job: &RealWorldJob, scoring: JobScoring) -> JobReport {
	let answer = produced_answer(job);
	let metrics = job_metrics(job, answer);
	let retrieval_quality = retrieval_quality_report(job, answer);

	JobReport {
		suite_id: job.suite.clone(),
		job_id: job.job_id.clone(),
		title: job.title.clone(),
		status: scoring.status,
		answer_type: job.expected_answer.answer_type.clone(),
		requires_caveat: job.expected_answer.requires_caveat,
		requires_refusal: job.expected_answer.requires_refusal,
		can_answer_unknown: job.allowed_uncertainty.can_answer_unknown,
		normalized_score: round3(scoring.normalized_score),
		hard_fail_hits: scoring.hard_fail_hits,
		expected_evidence: expected_evidence_report(job),
		produced_answer: answer.content.clone(),
		produced_evidence: produced_evidence_ids(answer).into_iter().collect(),
		unsupported_claim_count: scoring.unsupported_claims.len(),
		wrong_result_count: scoring.wrong_result_count,
		stale_answer_count: scoring
			.evolution
			.as_ref()
			.map_or(0, |report| report.stale_answer_count),
		conflict_detection_count: scoring
			.evolution
			.as_ref()
			.map_or(0, |report| report.conflict_detection_count),
		update_rationale_available: scoring
			.evolution
			.as_ref()
			.is_some_and(|report| report.update_rationale_available),
		temporal_validity_not_encoded: scoring
			.evolution
			.as_ref()
			.is_some_and(|report| report.temporal_validity_not_encoded),
		history_readback_encoded: scoring
			.evolution
			.as_ref()
			.is_some_and(|report| report.history_readback_encoded),
		retrieval_quality,
		latency_ms: answer.latency_ms,
		cost: answer.cost.clone(),
		trace_explainability: answer.trace_explainability.clone(),
		knowledge: scoring.knowledge,
		trap_ids_used: scoring.trap_ids_used,
		dimension_scores: scoring.dimension_scores,
		reason: scoring.reason,
		evidence_required_count: metrics.evidence_required_count,
		evidence_covered_count: metrics.evidence_covered_count,
		source_ref_required_count: metrics.source_ref_required_count,
		source_ref_covered_count: metrics.source_ref_covered_count,
		quote_required_count: metrics.quote_required_count,
		quote_covered_count: metrics.quote_covered_count,
		stale_retrieval_count: metrics.stale_retrieval_count,
		scope_check_count: metrics.scope_check_count,
		scope_correct_count: metrics.scope_correct_count,
		scope_violation_count: metrics.scope_violation_count,
		redaction_leak_count: metrics.redaction_leak_count,
		qdrant_rebuild_case: metrics.qdrant_rebuild_case,
		operator_debug: job.operator_debug.clone(),
		evolution: scoring.evolution,
		consolidation: scoring.consolidation,
	}
}

fn consolidation_job_report(job: &RealWorldJob) -> Option<ConsolidationJobReport> {
	let fixture = job.corpus.adapter_response.as_ref()?.consolidation.as_ref()?;
	let proposals = fixture.proposals.iter().map(consolidation_proposal_report).collect::<Vec<_>>();
	let executable_gaps = fixture
		.executable_gaps
		.iter()
		.map(|gap| ConsolidationExecutableGapReport {
			primitive: gap.primitive.clone(),
			follow_up_issue: gap.follow_up_issue.clone(),
			reason: gap.reason.clone(),
			blocks_fixture_pass: gap.blocks_fixture_pass,
		})
		.collect::<Vec<_>>();
	let proposal_count = proposals.len();
	let source_mutation_count =
		proposals.iter().map(|proposal| proposal.source_mutation_count).sum();
	let proposal_unsupported_claim_count =
		proposals.iter().map(|proposal| proposal.unsupported_claim_count).sum();

	Some(ConsolidationJobReport {
		proposal_count,
		proposal_usefulness: mean_proposal_metric(
			proposals.iter().map(|proposal| proposal.usefulness_score),
		),
		lineage_completeness: mean_proposal_metric(
			proposals.iter().map(|proposal| proposal.lineage_completeness),
		),
		review_action_correctness: mean_proposal_metric(
			proposals.iter().map(|proposal| if proposal.review_action_correct { 1.0 } else { 0.0 }),
		),
		source_mutation_count,
		proposal_unsupported_claim_count,
		executable_gaps,
		proposals,
	})
}

fn consolidation_proposal_report(
	proposal: &ConsolidationProposalFixture,
) -> ConsolidationProposalReport {
	ConsolidationProposalReport {
		proposal_id: proposal.proposal_id.clone(),
		proposal_kind: proposal.proposal_kind.clone(),
		usefulness_score: round3(proposal.usefulness_score),
		min_usefulness_score: round3(proposal.min_usefulness_score),
		lineage_completeness: round3(lineage_completeness(proposal)),
		expected_review_action: proposal.expected_review_action,
		actual_review_action: proposal.actual_review_action,
		review_action_correct: proposal.expected_review_action == proposal.actual_review_action,
		source_mutation_count: proposal.source_mutations.len()
			+ forbidden_diff_key_count(&proposal.diff),
		unsupported_claim_count: proposal
			.unsupported_claim_count
			.max(proposal.unsupported_claim_flags.len()),
	}
}

fn lineage_completeness(proposal: &ConsolidationProposalFixture) -> f64 {
	let expected = proposal.expected_source_refs.iter().collect::<BTreeSet<_>>();
	let actual = proposal.source_refs.iter().collect::<BTreeSet<_>>();
	let matched = expected.iter().filter(|source_ref| actual.contains(**source_ref)).count();

	matched as f64 / expected.len() as f64
}

fn forbidden_diff_key_count(value: &Value) -> usize {
	match value {
		Value::Object(map) => map
			.iter()
			.map(|(key, nested)| {
				usize::from(FORBIDDEN_SOURCE_MUTATION_KEYS.contains(&key.as_str()))
					+ forbidden_diff_key_count(nested)
			})
			.sum(),
		Value::Array(items) => items.iter().map(forbidden_diff_key_count).sum(),
		_ => 0,
	}
}

fn proposal_usefulness_failures(consolidation: Option<&ConsolidationJobReport>) -> usize {
	consolidation.map_or(0, |report| {
		report
			.proposals
			.iter()
			.filter(|proposal| proposal.usefulness_score < proposal.min_usefulness_score)
			.count()
	})
}

fn lineage_failures(consolidation: Option<&ConsolidationJobReport>) -> usize {
	consolidation.map_or(0, |report| {
		report.proposals.iter().filter(|proposal| proposal.lineage_completeness < 1.0).count()
	})
}

fn review_action_failures(consolidation: Option<&ConsolidationJobReport>) -> usize {
	consolidation.map_or(0, |report| {
		report.proposals.iter().filter(|proposal| !proposal.review_action_correct).count()
	})
}

fn blocking_executable_gaps(consolidation: Option<&ConsolidationJobReport>) -> usize {
	consolidation.map_or(0, |report| {
		report.executable_gaps.iter().filter(|gap| gap.blocks_fixture_pass).count()
	})
}

fn mean_proposal_metric(values: impl Iterator<Item = f64>) -> Option<f64> {
	let values = values.collect::<Vec<_>>();

	if values.is_empty() {
		None
	} else {
		Some(round3(values.iter().sum::<f64>() / values.len() as f64))
	}
}

fn job_metrics(job: &RealWorldJob, answer: &ProducedAnswer) -> JobMetrics {
	let produced_evidence = produced_evidence_ids(answer);
	let source_ref_by_evidence = source_ref_by_evidence(job);
	let evidence_required_count =
		job.required_evidence.iter().filter(|evidence| is_required_use(evidence)).count();
	let evidence_covered_count = job
		.required_evidence
		.iter()
		.filter(|evidence| is_required_use(evidence))
		.filter(|evidence| produced_evidence.contains(&evidence.evidence_id))
		.count();
	let source_ref_required_count = evidence_required_count;
	let source_ref_covered_count = job
		.required_evidence
		.iter()
		.filter(|evidence| is_required_use(evidence))
		.filter(|evidence| produced_evidence.contains(&evidence.evidence_id))
		.filter(|evidence| {
			source_ref_by_evidence.get(evidence.evidence_id.as_str()).is_some_and(|source_ref| {
				source_ref.as_object().is_some_and(|object| !object.is_empty())
			})
		})
		.count();
	let quote_required_count = job
		.required_evidence
		.iter()
		.filter(|evidence| is_required_use(evidence) && evidence.quote.is_some())
		.count();
	let quote_covered_count = job
		.required_evidence
		.iter()
		.filter(|evidence| is_required_use(evidence) && evidence.quote.is_some())
		.filter(|evidence| produced_evidence.contains(&evidence.evidence_id))
		.count();
	let stale_retrieval_count = trap_use_count(job, &produced_evidence, "stale_fact", answer);
	let scope_violation_count = trap_use_count(job, &produced_evidence, "near_duplicate", answer);
	let scope_check_count =
		job.negative_traps.iter().filter(|trap| trap.trap_type == "near_duplicate").count();
	let redaction_leak_count = trap_use_count(job, &produced_evidence, "privacy_leak", answer);
	let scope_correct_count = scope_check_count.saturating_sub(scope_violation_count);
	let qdrant_rebuild_case = job.tags.iter().any(|tag| tag == "qdrant_rebuild");

	JobMetrics {
		evidence_required_count,
		evidence_covered_count,
		source_ref_required_count,
		source_ref_covered_count,
		quote_required_count,
		quote_covered_count,
		stale_retrieval_count,
		scope_check_count,
		scope_correct_count,
		scope_violation_count,
		redaction_leak_count,
		qdrant_rebuild_case,
	}
}

fn source_ref_by_evidence(job: &RealWorldJob) -> BTreeMap<&str, &Value> {
	job.corpus.items.iter().map(|item| (item.evidence_id.as_str(), &item.source_ref)).collect()
}

fn trap_use_count(
	job: &RealWorldJob,
	produced_evidence: &BTreeSet<String>,
	trap_type: &str,
	answer: &ProducedAnswer,
) -> usize {
	job.negative_traps
		.iter()
		.filter(|trap| trap.failure_if_used && trap.trap_type == trap_type)
		.filter(|trap| trap_was_used(job, trap, produced_evidence, answer))
		.count()
}

fn trap_was_used(
	job: &RealWorldJob,
	trap: &NegativeTrap,
	produced_evidence: &BTreeSet<String>,
	answer: &ProducedAnswer,
) -> bool {
	trap.evidence_ids.iter().any(|evidence_id| {
		produced_evidence.contains(evidence_id)
			|| answer_contains_corpus_item(job, evidence_id, answer)
	})
}

fn answer_contains_corpus_item(
	job: &RealWorldJob,
	evidence_id: &str,
	answer: &ProducedAnswer,
) -> bool {
	job.corpus
		.items
		.iter()
		.find(|item| item.evidence_id == evidence_id)
		.and_then(|item| item.text.as_deref())
		.is_some_and(|text| !text.trim().is_empty() && answer.content.contains(text))
}

fn retrieval_quality_report(job: &RealWorldJob, answer: &ProducedAnswer) -> RetrievalQualityReport {
	let expected = expected_evidence_ids(job);
	let allowed = allowed_evidence_ids(job);
	let produced = produced_evidence_ids(answer);
	let trap_evidence = trap_evidence_ids(job);
	let expected_evidence_matched =
		expected.iter().filter(|evidence_id| produced.contains(evidence_id.as_str())).count();
	let irrelevant_context_count =
		produced.iter().filter(|evidence_id| !allowed.contains(evidence_id.as_str())).count();
	let trap_context_count =
		produced.iter().filter(|evidence_id| trap_evidence.contains(evidence_id.as_str())).count();

	RetrievalQualityReport {
		expected_evidence_total: expected.len(),
		expected_evidence_matched,
		expected_evidence_recall: ratio_or(expected_evidence_matched, expected.len(), 1.0),
		produced_evidence_total: produced.len(),
		irrelevant_context_count,
		irrelevant_context_ratio: ratio_or(irrelevant_context_count, produced.len(), 0.0),
		trap_context_count,
	}
}

fn expected_evidence_ids(job: &RealWorldJob) -> BTreeSet<String> {
	job.required_evidence
		.iter()
		.filter(|evidence| is_required_use(evidence))
		.map(|evidence| evidence.evidence_id.clone())
		.collect()
}

fn allowed_evidence_ids(job: &RealWorldJob) -> BTreeSet<String> {
	let mut allowed = expected_evidence_ids(job);

	for link in job.expected_answer.evidence_links.values() {
		allowed.extend(link.ids());
	}

	allowed
}

fn trap_evidence_ids(job: &RealWorldJob) -> BTreeSet<String> {
	job.negative_traps.iter().flat_map(|trap| trap.evidence_ids.iter().cloned()).collect()
}

fn expected_evidence_report(job: &RealWorldJob) -> Vec<ExpectedEvidenceReport> {
	job.required_evidence
		.iter()
		.map(|evidence| ExpectedEvidenceReport {
			evidence_id: evidence.evidence_id.clone(),
			claim_id: evidence.claim_id.clone(),
			requirement: evidence.requirement.clone(),
		})
		.collect()
}

fn suite_reports(jobs: &[JobReport]) -> Vec<SuiteReport> {
	SUITES.iter().map(|suite_id| suite_report(suite_id, jobs)).collect()
}

fn suite_report(suite_id: &str, jobs: &[JobReport]) -> SuiteReport {
	let suite_jobs = jobs.iter().filter(|job| job.suite_id == suite_id).collect::<Vec<_>>();

	if suite_jobs.is_empty() {
		return SuiteReport {
			suite_id: suite_id.to_string(),
			status: TypedStatus::NotEncoded,
			encoded_job_count: 0,
			score_mean: None,
			unsupported_claim_count: 0,
			wrong_result_count: 0,
			stale_answer_count: 0,
			conflict_detection_count: 0,
			update_rationale_available_count: 0,
			temporal_validity_not_encoded_count: 0,
			history_readback_encoded_count: 0,
			expected_evidence_recall: None,
			irrelevant_context_ratio: None,
			trace_explainability_count: 0,
			reason: NOT_ENCODED_REASON.to_string(),
		};
	}

	let status = aggregate_status(&suite_jobs);
	let score_sum = suite_jobs.iter().map(|job| job.normalized_score).sum::<f64>();
	let unsupported_claim_count = suite_jobs.iter().map(|job| job.unsupported_claim_count).sum();
	let wrong_result_count = suite_jobs.iter().map(|job| job.wrong_result_count).sum();
	let stale_answer_count = suite_jobs.iter().map(|job| job.stale_answer_count).sum();
	let conflict_detection_count = suite_jobs.iter().map(|job| job.conflict_detection_count).sum();
	let update_rationale_available_count =
		suite_jobs.iter().filter(|job| job.update_rationale_available).count();
	let temporal_validity_not_encoded_count =
		suite_jobs.iter().filter(|job| job.temporal_validity_not_encoded).count();
	let history_readback_encoded_count =
		suite_jobs.iter().filter(|job| job.history_readback_encoded).count();
	let trace_explainability_count =
		suite_jobs.iter().filter(|job| job.trace_explainability.is_some()).count();

	SuiteReport {
		suite_id: suite_id.to_string(),
		status,
		encoded_job_count: suite_jobs.len(),
		score_mean: Some(round3(score_sum / suite_jobs.len() as f64)),
		unsupported_claim_count,
		wrong_result_count,
		stale_answer_count,
		conflict_detection_count,
		update_rationale_available_count,
		temporal_validity_not_encoded_count,
		history_readback_encoded_count,
		expected_evidence_recall: Some(expected_evidence_recall_for_jobs(&suite_jobs)),
		irrelevant_context_ratio: Some(irrelevant_context_ratio_for_jobs(&suite_jobs)),
		trace_explainability_count,
		reason: suite_reason(status, suite_jobs.len()),
	}
}

fn aggregate_status(jobs: &[&JobReport]) -> TypedStatus {
	let statuses = jobs.iter().map(|job| job.status).collect::<BTreeSet<_>>();

	if statuses.contains(&TypedStatus::UnsupportedClaim) {
		TypedStatus::UnsupportedClaim
	} else if statuses.contains(&TypedStatus::LifecycleFail) {
		TypedStatus::LifecycleFail
	} else if statuses.contains(&TypedStatus::WrongResult) {
		TypedStatus::WrongResult
	} else if statuses.contains(&TypedStatus::Incomplete) {
		TypedStatus::Incomplete
	} else if statuses.contains(&TypedStatus::Blocked) {
		TypedStatus::Blocked
	} else if statuses.contains(&TypedStatus::NotEncoded) {
		TypedStatus::NotEncoded
	} else if statuses.contains(&TypedStatus::Pass) {
		TypedStatus::Pass
	} else {
		TypedStatus::NotEncoded
	}
}

fn suite_reason(status: TypedStatus, encoded_job_count: usize) -> String {
	match status {
		TypedStatus::Pass => format!("All {encoded_job_count} encoded job(s) passed."),
		TypedStatus::UnsupportedClaim =>
			"At least one encoded job produced an unsupported claim.".to_string(),
		TypedStatus::WrongResult => "At least one encoded job returned a wrong result.".to_string(),
		TypedStatus::LifecycleFail =>
			"At least one encoded lifecycle-scored job failed lifecycle behavior.".to_string(),
		TypedStatus::Incomplete => "At least one encoded job could not complete.".to_string(),
		TypedStatus::Blocked => "At least one encoded job is blocked.".to_string(),
		TypedStatus::NotEncoded =>
			if encoded_job_count == 0 {
				NOT_ENCODED_REASON.to_string()
			} else {
				"At least one encoded fixture declares a not_encoded limitation.".to_string()
			},
	}
}

fn report_summary(jobs: &[JobReport], suites: &[SuiteReport]) -> ReportSummary {
	let job_refs = jobs.iter().collect::<Vec<_>>();
	let evidence_required_count = jobs.iter().map(|job| job.evidence_required_count).sum();
	let evidence_covered_count = jobs.iter().map(|job| job.evidence_covered_count).sum();
	let source_ref_required_count = jobs.iter().map(|job| job.source_ref_required_count).sum();
	let source_ref_covered_count = jobs.iter().map(|job| job.source_ref_covered_count).sum();
	let quote_required_count = jobs.iter().map(|job| job.quote_required_count).sum();
	let quote_covered_count = jobs.iter().map(|job| job.quote_covered_count).sum();
	let scope_check_count = jobs.iter().map(|job| job.scope_check_count).sum();
	let scope_correct_count = jobs.iter().map(|job| job.scope_correct_count).sum();
	let mut summary = ReportSummary {
		job_count: jobs.len(),
		encoded_suite_count: suites.iter().filter(|suite| suite.encoded_job_count > 0).count(),
		not_encoded: 0,
		unsupported_claim_count: jobs.iter().map(|job| job.unsupported_claim_count).sum(),
		wrong_result_count: jobs.iter().map(|job| job.wrong_result_count).sum(),
		stale_answer_count: jobs.iter().map(|job| job.stale_answer_count).sum(),
		conflict_detection_count: jobs.iter().map(|job| job.conflict_detection_count).sum(),
		update_rationale_available_count: jobs
			.iter()
			.filter(|job| job.update_rationale_available)
			.count(),
		temporal_validity_not_encoded_count: jobs
			.iter()
			.filter(|job| job.temporal_validity_not_encoded)
			.count(),
		history_readback_encoded_count: jobs
			.iter()
			.filter(|job| job.history_readback_encoded)
			.count(),
		expected_evidence_total: jobs
			.iter()
			.map(|job| job.retrieval_quality.expected_evidence_total)
			.sum(),
		expected_evidence_matched: jobs
			.iter()
			.map(|job| job.retrieval_quality.expected_evidence_matched)
			.sum(),
		expected_evidence_recall: expected_evidence_recall_for_jobs(&job_refs),
		irrelevant_context_count: jobs
			.iter()
			.map(|job| job.retrieval_quality.irrelevant_context_count)
			.sum(),
		irrelevant_context_ratio: irrelevant_context_ratio_for_jobs(&job_refs),
		trace_explainability_count: jobs
			.iter()
			.filter(|job| job.trace_explainability.is_some())
			.count(),
		wrong_result_stage_attribution_count: jobs
			.iter()
			.filter(|job| {
				job.status == TypedStatus::WrongResult
					&& trace_failure_stage(job.trace_explainability.as_ref()).is_some()
			})
			.count(),
		mean_score: mean_score(jobs),
		mean_latency_ms: mean_latency(jobs),
		total_cost: total_cost(jobs),
		evidence_required_count,
		evidence_covered_count,
		evidence_coverage: ratio(evidence_covered_count, evidence_required_count),
		source_ref_required_count,
		source_ref_covered_count,
		source_ref_coverage: ratio(source_ref_covered_count, source_ref_required_count),
		quote_required_count,
		quote_covered_count,
		quote_coverage: ratio(quote_covered_count, quote_required_count),
		stale_retrieval_count: jobs.iter().map(|job| job.stale_retrieval_count).sum(),
		scope_check_count,
		scope_correct_count,
		scope_correctness: ratio(scope_correct_count, scope_check_count),
		scope_violation_count: jobs.iter().map(|job| job.scope_violation_count).sum(),
		redaction_leak_count: jobs.iter().map(|job| job.redaction_leak_count).sum(),
		qdrant_rebuild_case_count: jobs.iter().filter(|job| job.qdrant_rebuild_case).count(),
		qdrant_rebuild_pass_count: jobs
			.iter()
			.filter(|job| job.qdrant_rebuild_case && job.status == TypedStatus::Pass)
			.count(),
		operator_debug_job_count: jobs.iter().filter(|job| job.operator_debug.is_some()).count(),
		raw_sql_needed_count: jobs
			.iter()
			.filter_map(|job| job.operator_debug.as_ref())
			.filter(|debug| debug.raw_sql_needed)
			.count(),
		trace_incomplete_count: jobs
			.iter()
			.filter_map(|job| job.operator_debug.as_ref())
			.filter(|debug| debug.trace_completeness != "complete")
			.count(),
		operator_ux_gap_count: jobs
			.iter()
			.filter_map(|job| job.operator_debug.as_ref())
			.map(|debug| debug.ux_gaps.len())
			.sum(),
		consolidation: consolidation_summary(jobs),
		knowledge: knowledge_summary(jobs),
		..ReportSummary::default()
	};

	for job in jobs {
		match job.status {
			TypedStatus::Pass => summary.pass += 1,
			TypedStatus::WrongResult => summary.wrong_result += 1,
			TypedStatus::LifecycleFail => summary.lifecycle_fail += 1,
			TypedStatus::Incomplete => summary.incomplete += 1,
			TypedStatus::Blocked => summary.blocked += 1,
			TypedStatus::NotEncoded => summary.not_encoded += 1,
			TypedStatus::UnsupportedClaim => summary.unsupported_claim += 1,
		}
	}

	summary
}

fn evolution_summary(jobs: &[JobReport]) -> EvolutionSummary {
	EvolutionSummary {
		stale_answer_count: jobs.iter().map(|job| job.stale_answer_count).sum(),
		conflict_detection_count: jobs.iter().map(|job| job.conflict_detection_count).sum(),
		update_rationale_available_count: jobs
			.iter()
			.filter(|job| job.update_rationale_available)
			.count(),
		temporal_validity_not_encoded_count: jobs
			.iter()
			.filter(|job| job.temporal_validity_not_encoded)
			.count(),
		history_readback_encoded_count: jobs
			.iter()
			.filter(|job| job.history_readback_encoded)
			.count(),
	}
}

fn follow_up_reports(jobs: &[RealWorldJob]) -> Vec<FollowUpReport> {
	jobs.iter()
		.filter_map(|job| {
			job.encoding.follow_up.as_ref().map(|follow_up| FollowUpReport {
				suite_id: job.suite.clone(),
				job_id: job.job_id.clone(),
				title: follow_up.title.clone(),
				reason: follow_up.reason.clone(),
			})
		})
		.collect()
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
	if denominator == 0 {
		return 0.0;
	}

	round3(numerator as f64 / denominator as f64)
}

fn expected_evidence_recall_for_jobs(jobs: &[&JobReport]) -> f64 {
	let total = jobs.iter().map(|job| job.retrieval_quality.expected_evidence_total).sum::<usize>();
	let matched =
		jobs.iter().map(|job| job.retrieval_quality.expected_evidence_matched).sum::<usize>();

	ratio_or(matched, total, 1.0)
}

fn irrelevant_context_ratio_for_jobs(jobs: &[&JobReport]) -> f64 {
	let total = jobs.iter().map(|job| job.retrieval_quality.produced_evidence_total).sum::<usize>();
	let irrelevant =
		jobs.iter().map(|job| job.retrieval_quality.irrelevant_context_count).sum::<usize>();

	ratio_or(irrelevant, total, 0.0)
}

fn ratio_or(numerator: usize, denominator: usize, empty_value: f64) -> f64 {
	if denominator == 0 { empty_value } else { round3(numerator as f64 / denominator as f64) }
}

fn ratio_or_full(numerator: usize, denominator: usize) -> f64 {
	ratio_or(numerator, denominator, 1.0)
}

fn consolidation_summary(jobs: &[JobReport]) -> ConsolidationSummaryReport {
	let reports = jobs.iter().filter_map(|job| job.consolidation.as_ref()).collect::<Vec<_>>();

	if reports.is_empty() {
		return ConsolidationSummaryReport::default();
	}

	let proposals = reports.iter().flat_map(|report| report.proposals.iter()).collect::<Vec<_>>();
	let executable_gap_count = reports.iter().map(|report| report.executable_gaps.len()).sum();

	ConsolidationSummaryReport {
		proposal_count: proposals.len(),
		proposal_usefulness: mean_proposal_metric(
			proposals.iter().map(|proposal| proposal.usefulness_score),
		),
		lineage_completeness: mean_proposal_metric(
			proposals.iter().map(|proposal| proposal.lineage_completeness),
		),
		review_action_correctness: mean_proposal_metric(
			proposals.iter().map(|proposal| if proposal.review_action_correct { 1.0 } else { 0.0 }),
		),
		source_mutation_count: proposals
			.iter()
			.map(|proposal| proposal.source_mutation_count)
			.sum(),
		proposal_unsupported_claim_count: proposals
			.iter()
			.map(|proposal| proposal.unsupported_claim_count)
			.sum(),
		executable_gap_count,
	}
}

fn knowledge_summary(jobs: &[JobReport]) -> Option<KnowledgeSummary> {
	let knowledge_jobs = jobs.iter().filter_map(|job| job.knowledge.as_ref()).collect::<Vec<_>>();

	if knowledge_jobs.is_empty() {
		return None;
	}

	let job_count = knowledge_jobs.len();
	let page_count = knowledge_jobs.iter().map(|metrics| metrics.page_count).sum::<usize>();
	let section_count = knowledge_jobs.iter().map(|metrics| metrics.section_count).sum::<usize>();
	let traced_section_count =
		knowledge_jobs.iter().map(|metrics| metrics.traced_section_count).sum::<usize>();
	let stale_trap_count =
		knowledge_jobs.iter().map(|metrics| metrics.stale_trap_count).sum::<usize>();
	let stale_traps_detected =
		knowledge_jobs.iter().map(|metrics| metrics.stale_traps_detected).sum::<usize>();
	let deterministic_rebuild_count =
		knowledge_jobs.iter().map(|metrics| metrics.deterministic_rebuild_count).sum::<usize>();
	let rebuild_page_count =
		knowledge_jobs.iter().map(|metrics| metrics.rebuild_page_count).sum::<usize>();
	let backlink_count = knowledge_jobs.iter().map(|metrics| metrics.backlink_count).sum::<usize>();
	let pages_with_backlinks =
		knowledge_jobs.iter().map(|metrics| metrics.pages_with_backlinks).sum::<usize>();
	let page_usefulness = round3(
		knowledge_jobs.iter().map(|metrics| metrics.page_usefulness).sum::<f64>()
			/ job_count as f64,
	);

	Some(KnowledgeSummary {
		job_count,
		page_count,
		section_count,
		backlink_count,
		pages_with_backlinks,
		citation_coverage: ratio(traced_section_count, section_count),
		stale_claim_detection: ratio_or_full(stale_traps_detected, stale_trap_count),
		rebuild_determinism: ratio(deterministic_rebuild_count, rebuild_page_count),
		backlink_coverage: ratio(pages_with_backlinks, page_count),
		page_usefulness,
		unsupported_summary_count: knowledge_jobs
			.iter()
			.map(|metrics| metrics.unsupported_summary_count)
			.sum(),
		untraced_section_count: knowledge_jobs
			.iter()
			.map(|metrics| metrics.untraced_section_count)
			.sum(),
		allowed_variance_count: knowledge_jobs
			.iter()
			.map(|metrics| metrics.allowed_variance_count)
			.sum(),
	})
}

fn mean_score(jobs: &[JobReport]) -> f64 {
	if jobs.is_empty() {
		return 0.0;
	}

	round3(jobs.iter().map(|job| job.normalized_score).sum::<f64>() / jobs.len() as f64)
}

fn mean_latency(jobs: &[JobReport]) -> Option<f64> {
	let latencies = jobs.iter().filter_map(|job| job.latency_ms).collect::<Vec<_>>();

	if latencies.is_empty() {
		return None;
	}

	Some(round3(latencies.iter().sum::<f64>() / latencies.len() as f64))
}

fn total_cost(jobs: &[JobReport]) -> Option<CostReport> {
	let costs = jobs.iter().filter_map(|job| job.cost.as_ref()).collect::<Vec<_>>();

	if costs.is_empty() {
		return None;
	}

	let currency = costs.iter().find_map(|cost| cost.currency.clone());
	let amount = sum_optional_f64(costs.iter().filter_map(|cost| cost.amount));
	let input_tokens = sum_optional_u64(costs.iter().filter_map(|cost| cost.input_tokens));
	let output_tokens = sum_optional_u64(costs.iter().filter_map(|cost| cost.output_tokens));

	Some(CostReport { currency, amount, input_tokens, output_tokens })
}

fn sum_optional_f64(values: impl Iterator<Item = f64>) -> Option<f64> {
	let values = values.collect::<Vec<_>>();

	if values.is_empty() { None } else { Some(round3(values.iter().sum())) }
}

fn sum_optional_u64(values: impl Iterator<Item = u64>) -> Option<u64> {
	let values = values.collect::<Vec<_>>();

	if values.is_empty() { None } else { Some(values.iter().sum()) }
}

fn corpus_profile(jobs: &[RealWorldJob]) -> String {
	let profiles = jobs.iter().map(|job| job.corpus.profile.as_str()).collect::<BTreeSet<_>>();

	if profiles.len() == 1 {
		profiles.into_iter().next().unwrap_or("unknown").to_string()
	} else {
		"mixed".to_string()
	}
}

fn adapter_report(args: &RunArgs) -> Result<AdapterReport> {
	Ok(AdapterReport {
		adapter_id: args.adapter_id.clone(),
		name: args.adapter_name.clone(),
		behavior: args.adapter_behavior.clone(),
		storage: typed_status_from_arg(
			args.adapter_storage_status.as_str(),
			"--adapter-storage-status",
		)?,
		runtime: typed_status_from_arg(
			args.adapter_runtime_status.as_str(),
			"--adapter-runtime-status",
		)?,
		notes: args.adapter_notes.clone(),
	})
}

fn typed_status_from_arg(raw: &str, flag: &str) -> Result<TypedStatus> {
	match raw {
		"pass" => Ok(TypedStatus::Pass),
		"wrong_result" => Ok(TypedStatus::WrongResult),
		"lifecycle_fail" => Ok(TypedStatus::LifecycleFail),
		"incomplete" => Ok(TypedStatus::Incomplete),
		"blocked" => Ok(TypedStatus::Blocked),
		"not_encoded" => Ok(TypedStatus::NotEncoded),
		"unsupported_claim" => Ok(TypedStatus::UnsupportedClaim),
		_ => Err(eyre::eyre!(
			"{flag} must be one of pass, wrong_result, lifecycle_fail, incomplete, blocked, not_encoded, or unsupported_claim."
		)),
	}
}

fn external_adapter_section(
	manifest_path: &Path,
	skip_manifest: bool,
) -> Result<ExternalAdapterSection> {
	if skip_manifest {
		return Ok(empty_external_adapter_section("skipped"));
	}

	let manifest_path = resolve_external_adapter_manifest_path(manifest_path);

	if !manifest_path.exists() {
		return Ok(empty_external_adapter_section("missing"));
	}

	let raw = fs::read_to_string(&manifest_path)?;
	let manifest = serde_json::from_str::<ExternalAdapterManifest>(&raw).map_err(|err| {
		eyre::eyre!("Failed to parse external adapter manifest {}: {err}", manifest_path.display())
	})?;

	validate_external_adapter_manifest(&manifest, &manifest_path)?;

	let summary = external_adapter_summary(&manifest.adapters);

	Ok(ExternalAdapterSection {
		schema: EXTERNAL_ADAPTER_REPORT_SCHEMA.to_string(),
		manifest_id: manifest.manifest_id,
		docker_isolation: manifest.docker_isolation,
		summary,
		adapters: manifest.adapters,
	})
}

fn empty_external_adapter_section(reason: &str) -> ExternalAdapterSection {
	ExternalAdapterSection {
		schema: EXTERNAL_ADAPTER_REPORT_SCHEMA.to_string(),
		manifest_id: reason.to_string(),
		docker_isolation: ExternalDockerIsolation::default(),
		summary: ExternalAdapterSummary::default(),
		adapters: Vec::new(),
	}
}

fn resolve_external_adapter_manifest_path(path: &Path) -> PathBuf {
	if path.exists() || path.is_absolute() {
		return path.to_path_buf();
	}

	let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
	let Some(workspace_root) = manifest_dir.parent().and_then(Path::parent) else {
		return path.to_path_buf();
	};
	let workspace_candidate = workspace_root.join(path);

	if workspace_candidate.exists() { workspace_candidate } else { path.to_path_buf() }
}

fn validate_external_adapter_manifest(
	manifest: &ExternalAdapterManifest,
	path: &Path,
) -> Result<()> {
	if manifest.schema != EXTERNAL_ADAPTER_MANIFEST_SCHEMA {
		return Err(eyre::eyre!(
			"{} has schema {}, expected {EXTERNAL_ADAPTER_MANIFEST_SCHEMA}.",
			path.display(),
			manifest.schema
		));
	}
	if manifest.manifest_id.trim().is_empty() {
		return Err(eyre::eyre!("{} has an empty manifest_id.", path.display()));
	}

	validate_external_docker_isolation(path, &manifest.docker_isolation)?;

	validate_external_adapters(path, &manifest.adapters)
}

fn validate_external_docker_isolation(path: &Path, docker: &ExternalDockerIsolation) -> Result<()> {
	if docker.compose_file.trim().is_empty()
		|| docker.runner.trim().is_empty()
		|| docker.artifact_dir.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has incomplete docker_isolation metadata.", path.display()));
	}
	if !docker.default {
		return Err(eyre::eyre!(
			"{} external adapter manifest must default to Docker isolation.",
			path.display()
		));
	}
	if docker.host_global_installs_required {
		return Err(eyre::eyre!(
			"{} external adapter manifest must not require host-global installs by default.",
			path.display()
		));
	}

	Ok(())
}

fn validate_external_adapters(path: &Path, adapters: &[ExternalAdapterReport]) -> Result<()> {
	if adapters.is_empty() {
		return Err(eyre::eyre!("{} declares no external adapters.", path.display()));
	}

	let mut seen = BTreeSet::new();

	for adapter in adapters {
		validate_external_adapter(path, adapter)?;

		if !seen.insert(adapter.adapter_id.as_str()) {
			return Err(eyre::eyre!(
				"{} declares duplicate adapter_id {}.",
				path.display(),
				adapter.adapter_id
			));
		}
	}

	Ok(())
}

fn validate_external_adapter(path: &Path, adapter: &ExternalAdapterReport) -> Result<()> {
	if adapter.adapter_id.trim().is_empty()
		|| adapter.project.trim().is_empty()
		|| adapter.adapter_kind.trim().is_empty()
		|| adapter.evidence_class.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete external adapter.", path.display()));
	}
	if !matches!(
		adapter.evidence_class.as_str(),
		"fixture_backed" | "live_baseline_only" | "live_real_world" | "research_gate"
	) {
		return Err(eyre::eyre!(
			"{} adapter {} has unsupported evidence_class {}.",
			path.display(),
			adapter.adapter_id,
			adapter.evidence_class
		));
	}
	if adapter.docker_default && adapter.host_global_installs_required {
		return Err(eyre::eyre!(
			"{} adapter {} is Docker-default but requires host-global installs.",
			path.display(),
			adapter.adapter_id
		));
	}

	validate_adapter_execution(path, adapter)?;
	validate_adapter_capabilities(path, adapter)?;
	validate_adapter_suites(path, adapter)?;
	validate_adapter_scenarios(path, adapter)?;
	validate_adapter_evidence(path, adapter)?;
	validate_adapter_execution_metadata(path, adapter)?;

	if let Some(follow_up) = &adapter.follow_up
		&& (follow_up.title.trim().is_empty() || follow_up.reason.trim().is_empty())
	{
		return Err(eyre::eyre!(
			"{} adapter {} has an incomplete follow_up.",
			path.display(),
			adapter.adapter_id
		));
	}

	Ok(())
}

fn validate_adapter_execution(path: &Path, adapter: &ExternalAdapterReport) -> Result<()> {
	for evidence in [&adapter.setup, &adapter.run, &adapter.result] {
		if evidence.evidence.trim().is_empty()
			|| evidence.command.as_deref().is_some_and(str::is_empty)
			|| evidence.artifact.as_deref().is_some_and(str::is_empty)
		{
			return Err(eyre::eyre!(
				"{} adapter {} has incomplete setup/run/result evidence.",
				path.display(),
				adapter.adapter_id
			));
		}
	}

	Ok(())
}

fn validate_adapter_capabilities(path: &Path, adapter: &ExternalAdapterReport) -> Result<()> {
	for capability in &adapter.capabilities {
		if capability.capability.trim().is_empty() || capability.evidence.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} adapter {} has incomplete capability coverage.",
				path.display(),
				adapter.adapter_id
			));
		}
	}

	Ok(())
}

fn validate_adapter_suites(path: &Path, adapter: &ExternalAdapterReport) -> Result<()> {
	for suite in &adapter.suites {
		if !SUITES.contains(&suite.suite_id.as_str()) {
			return Err(eyre::eyre!(
				"{} adapter {} references unknown suite {}.",
				path.display(),
				adapter.adapter_id,
				suite.suite_id
			));
		}
		if suite.evidence.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} adapter {} has suite {} without evidence.",
				path.display(),
				adapter.adapter_id,
				suite.suite_id
			));
		}
	}

	Ok(())
}

fn validate_adapter_scenarios(path: &Path, adapter: &ExternalAdapterReport) -> Result<()> {
	for scenario in &adapter.scenarios {
		if scenario.scenario_id.trim().is_empty()
			|| scenario.evidence.trim().is_empty()
			|| scenario.command.as_deref().is_some_and(str::is_empty)
			|| scenario.artifact.as_deref().is_some_and(str::is_empty)
		{
			return Err(eyre::eyre!(
				"{} adapter {} has incomplete scenario judgment.",
				path.display(),
				adapter.adapter_id
			));
		}

		if let Some(suite_id) = &scenario.suite_id
			&& !SUITES.contains(&suite_id.as_str())
		{
			return Err(eyre::eyre!(
				"{} adapter {} scenario {} references unknown suite {}.",
				path.display(),
				adapter.adapter_id,
				scenario.scenario_id,
				suite_id
			));
		}
	}

	Ok(())
}

fn validate_adapter_evidence(path: &Path, adapter: &ExternalAdapterReport) -> Result<()> {
	for evidence in &adapter.evidence {
		if evidence.kind.trim().is_empty() || evidence.reference.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} adapter {} has incomplete evidence pointers.",
				path.display(),
				adapter.adapter_id
			));
		}
	}

	Ok(())
}

fn validate_adapter_execution_metadata(path: &Path, adapter: &ExternalAdapterReport) -> Result<()> {
	let Some(metadata) = &adapter.execution_metadata else {
		return Ok(());
	};

	if metadata.setup_path.trim().is_empty()
		|| metadata.runtime_boundary.trim().is_empty()
		|| metadata.resource_expectation.trim().is_empty()
		|| metadata.retry_guidance.iter().any(|guidance| guidance.trim().is_empty())
		|| metadata.sources.is_empty()
	{
		return Err(eyre::eyre!(
			"{} adapter {} has incomplete execution metadata.",
			path.display(),
			adapter.adapter_id
		));
	}

	for source in &metadata.sources {
		if source.label.trim().is_empty()
			|| source.url.trim().is_empty()
			|| source.evidence.trim().is_empty()
		{
			return Err(eyre::eyre!(
				"{} adapter {} has incomplete source metadata.",
				path.display(),
				adapter.adapter_id
			));
		}
	}

	Ok(())
}

fn external_adapter_summary(adapters: &[ExternalAdapterReport]) -> ExternalAdapterSummary {
	let external_projects = adapters
		.iter()
		.filter_map(|adapter| (adapter.project != "ELF").then_some(adapter.project.as_str()))
		.collect::<BTreeSet<_>>();
	let mut summary = ExternalAdapterSummary {
		adapter_count: adapters.len(),
		external_project_count: external_projects.len(),
		..ExternalAdapterSummary::default()
	};

	for adapter in adapters {
		accumulate_adapter_summary(&mut summary, adapter);
	}

	summary
}

fn accumulate_adapter_summary(
	summary: &mut ExternalAdapterSummary,
	adapter: &ExternalAdapterReport,
) {
	summary.docker_default_count += usize::from(adapter.docker_default);
	summary.host_global_install_required_count +=
		usize::from(adapter.host_global_installs_required);
	summary.fixture_backed_count += usize::from(adapter.evidence_class == "fixture_backed");
	summary.live_baseline_only_count += usize::from(adapter.evidence_class == "live_baseline_only");
	summary.live_real_world_count += usize::from(adapter.evidence_class == "live_real_world");
	summary.research_gate_count += usize::from(adapter.evidence_class == "research_gate");

	increment_adapter_status_count(&mut summary.overall_status_counts, adapter.overall_status);

	for capability in &adapter.capabilities {
		increment_adapter_status_count(&mut summary.capability_status_counts, capability.status);
	}
	for suite in &adapter.suites {
		increment_adapter_status_count(&mut summary.suite_status_counts, suite.status);
	}
	for scenario in &adapter.scenarios {
		increment_adapter_status_count(&mut summary.scenario_status_counts, scenario.status);
		increment_scenario_position_count(
			&mut summary.scenario_position_counts,
			scenario.elf_position,
		);
		increment_scenario_outcome_count(
			&mut summary.scenario_outcome_counts,
			scenario_comparison_outcome(scenario),
		);
	}
}

fn increment_adapter_status_count(counts: &mut AdapterStatusCounts, status: AdapterCoverageStatus) {
	match status {
		AdapterCoverageStatus::Real => counts.real += 1,
		AdapterCoverageStatus::Mocked => counts.mocked += 1,
		AdapterCoverageStatus::Unsupported => counts.unsupported += 1,
		AdapterCoverageStatus::Blocked => counts.blocked += 1,
		AdapterCoverageStatus::Incomplete => counts.incomplete += 1,
		AdapterCoverageStatus::WrongResult => counts.wrong_result += 1,
		AdapterCoverageStatus::LifecycleFail => counts.lifecycle_fail += 1,
		AdapterCoverageStatus::Pass => counts.pass += 1,
		AdapterCoverageStatus::NotEncoded => counts.not_encoded += 1,
	}
}

fn increment_scenario_position_count(
	counts: &mut ScenarioPositionCounts,
	position: ElfScenarioPosition,
) {
	match position {
		ElfScenarioPosition::Wins => counts.wins += 1,
		ElfScenarioPosition::Ties => counts.ties += 1,
		ElfScenarioPosition::Loses => counts.loses += 1,
		ElfScenarioPosition::Untested => counts.untested += 1,
	}
}

fn scenario_comparison_outcome(scenario: &AdapterScenarioJudgment) -> ScenarioComparisonOutcome {
	scenario.comparison_outcome.unwrap_or(match scenario.elf_position {
		ElfScenarioPosition::Wins => ScenarioComparisonOutcome::Win,
		ElfScenarioPosition::Ties => ScenarioComparisonOutcome::Tie,
		ElfScenarioPosition::Loses => ScenarioComparisonOutcome::Loss,
		ElfScenarioPosition::Untested => ScenarioComparisonOutcome::NotTested,
	})
}

fn increment_scenario_outcome_count(
	counts: &mut ScenarioOutcomeCounts,
	outcome: ScenarioComparisonOutcome,
) {
	match outcome {
		ScenarioComparisonOutcome::Win => counts.win += 1,
		ScenarioComparisonOutcome::Tie => counts.tie += 1,
		ScenarioComparisonOutcome::Loss => counts.loss += 1,
		ScenarioComparisonOutcome::NotTested => counts.not_tested += 1,
		ScenarioComparisonOutcome::Blocked => counts.blocked += 1,
		ScenarioComparisonOutcome::NonGoal => counts.non_goal += 1,
	}
}

fn capture_integration_report(jobs: &[RealWorldJob]) -> CaptureIntegrationReport {
	let mut report = CaptureIntegrationReport::default();

	for job in jobs {
		extend_unique(&mut report.real, &job.corpus.capture_behaviors.real);
		extend_unique(&mut report.fixture_backed, &job.corpus.capture_behaviors.fixture_backed);
		extend_unique(&mut report.mocked, &job.corpus.capture_behaviors.mocked);
		extend_unique(&mut report.blocked, &job.corpus.capture_behaviors.blocked);
		extend_unique(&mut report.not_encoded, &job.corpus.capture_behaviors.not_encoded);
		extend_unique(&mut report.notes, &job.corpus.capture_behaviors.notes);
	}

	if report.real.is_empty()
		&& report.fixture_backed.is_empty()
		&& report.mocked.is_empty()
		&& report.blocked.is_empty()
		&& report.not_encoded.is_empty()
	{
		report
			.not_encoded
			.push("No capture/integration behavior was declared by encoded fixtures.".to_string());
	}

	report
}

fn extend_unique(target: &mut Vec<String>, values: &[String]) {
	let mut seen = target.iter().cloned().collect::<BTreeSet<_>>();

	for value in values {
		if seen.insert(value.clone()) {
			target.push(value.clone());
		}
	}
}

fn private_corpus_redaction(jobs: &[RealWorldJob]) -> PrivateCorpusRedaction {
	let private_fixture_count = jobs
		.iter()
		.filter(|job| matches!(job.corpus.profile, CorpusProfile::PrivateSanitized))
		.count();
	let policy = if private_fixture_count == 0 {
		"no_private_corpus".to_string()
	} else {
		"publish evidence ids and bounded score summaries only; do not publish private text"
			.to_string()
	};

	PrivateCorpusRedaction { policy, private_fixture_count }
}

fn render_markdown(report: &RealWorldReport, report_path: &Path) -> String {
	let report_path = report_path.display().to_string();
	let mut out = String::new();

	render_markdown_header(&mut out, report, report_path.as_str());
	render_markdown_external_adapters(&mut out, report);
	render_markdown_capture_integration(&mut out, report);
	render_markdown_suites(&mut out, report);
	render_markdown_jobs(&mut out, report);
	render_markdown_operator_debugging(&mut out, report);
	render_markdown_evolution(&mut out, report);
	render_markdown_trace_explainability(&mut out, report);
	render_markdown_consolidation(&mut out, report);
	render_markdown_knowledge(&mut out, report);
	render_markdown_unsupported_claims(&mut out, report);
	render_markdown_follow_ups(&mut out, report);
	render_markdown_semantics(&mut out, report);

	out
}

fn render_markdown_capture_integration(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Capture And Integration Coverage\n\n");

	if report.adapter.behavior == DEFAULT_ADAPTER_BEHAVIOR {
		out.push_str("The real-world job runner is fixture-backed. This section separates encoded evidence from live adapter claims.\n\n");
	} else {
		out.push_str("This report scores materialized adapter responses. Capture and integration classes still describe the job corpus, not broad external adapter coverage.\n\n");
	}

	out.push_str("| Class | Behaviors |\n");
	out.push_str("| --- | --- |\n");
	out.push_str(&format!("| real | {} |\n", md_list(report.capture_integration.real.as_slice())));
	out.push_str(&format!(
		"| fixture-backed | {} |\n",
		md_list(report.capture_integration.fixture_backed.as_slice())
	));
	out.push_str(&format!(
		"| mocked | {} |\n",
		md_list(report.capture_integration.mocked.as_slice())
	));
	out.push_str(&format!(
		"| blocked | {} |\n",
		md_list(report.capture_integration.blocked.as_slice())
	));
	out.push_str(&format!(
		"| not encoded | {} |\n",
		md_list(report.capture_integration.not_encoded.as_slice())
	));

	if !report.capture_integration.notes.is_empty() {
		out.push_str("\nNotes:\n");

		for note in &report.capture_integration.notes {
			out.push_str(&format!("- {}\n", md_cell(note.as_str())));
		}
	}

	out.push('\n');
}

fn render_markdown_external_adapters(out: &mut String, report: &RealWorldReport) {
	out.push_str("## External Adapter Coverage\n\n");

	if report.external_adapters.adapters.is_empty() {
		out.push_str("No external adapter coverage manifest was loaded for this report.\n\n");

		return;
	}

	let summary = &report.external_adapters.summary;

	out.push_str("This section is manifest-backed. It records external adapter coverage and blockers, but it does not convert live-baseline retrieval results into real-world suite wins.\n\n");
	out.push_str(&format!(
		"- Manifest: `{}`\n",
		md_inline(report.external_adapters.manifest_id.as_str())
	));
	out.push_str(&format!(
		"- Docker default: `{}` via `{}`; artifact dir `{}`\n",
		report.external_adapters.docker_isolation.default,
		md_inline(report.external_adapters.docker_isolation.compose_file.as_str()),
		md_inline(report.external_adapters.docker_isolation.artifact_dir.as_str())
	));
	out.push_str(&format!(
		"- Adapter records: `{}` total, `{}` external project(s), `{}` Docker-default, `{}` requiring host-global installs\n",
		summary.adapter_count,
		summary.external_project_count,
		summary.docker_default_count,
		summary.host_global_install_required_count
	));
	out.push_str(&format!(
		"- Evidence classes: `{}` fixture-backed, `{}` live-baseline-only, `{}` live real-world, `{}` research-gate\n",
		summary.fixture_backed_count,
		summary.live_baseline_only_count,
		summary.live_real_world_count,
		summary.research_gate_count
	));
	out.push_str(&format!(
		"- Overall statuses: `{}`\n",
		adapter_status_counts_display(&summary.overall_status_counts)
	));
	out.push_str(&format!(
		"- Capability coverage statuses: `{}`\n",
		adapter_status_counts_display(&summary.capability_status_counts)
	));
	out.push_str(&format!(
		"- Real-world suite statuses: `{}`\n",
		adapter_status_counts_display(&summary.suite_status_counts)
	));

	if has_adapter_scenarios(report.external_adapters.adapters.as_slice()) {
		out.push_str(&format!(
			"- Scenario coverage statuses: `{}`\n",
			adapter_status_counts_display(&summary.scenario_status_counts)
		));
		out.push_str(&format!(
			"- ELF scenario positions: `{}`\n",
			scenario_position_counts_display(&summary.scenario_position_counts)
		));
		out.push_str(&format!(
			"- Scenario comparison outcomes: `{}`\n",
			scenario_outcome_counts_display(&summary.scenario_outcome_counts)
		));
	}

	out.push('\n');
	out.push_str("| Project | Adapter | Evidence Class | Overall | Setup | Run | Result | Docker | Suites | Evidence |\n");
	out.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");

	for adapter in &report.external_adapters.adapters {
		out.push_str(&format!(
			"| {} | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | {} | {} |\n",
			md_cell(adapter.project.as_str()),
			md_inline(adapter.adapter_id.as_str()),
			md_inline(adapter.evidence_class.as_str()),
			adapter_status_str(adapter.overall_status),
			adapter_status_str(adapter.setup.status),
			adapter_status_str(adapter.run.status),
			adapter_status_str(adapter.result.status),
			adapter.docker_default,
			adapter_suite_cell(adapter.suites.as_slice()),
			adapter_evidence_cell(adapter)
		));
	}

	out.push_str("\n### Adapter Capability Details\n\n");
	out.push_str("| Adapter | Capability | Status | Evidence |\n");
	out.push_str("| --- | --- | --- | --- |\n");

	for adapter in &report.external_adapters.adapters {
		for capability in &adapter.capabilities {
			out.push_str(&format!(
				"| `{}` | {} | `{}` | {} |\n",
				md_inline(adapter.adapter_id.as_str()),
				md_cell(capability.capability.as_str()),
				adapter_status_str(capability.status),
				md_cell(capability.evidence.as_str())
			));
		}
	}

	render_markdown_adapter_scenarios(out, report.external_adapters.adapters.as_slice());
	render_markdown_adapter_execution_metadata(out, report.external_adapters.adapters.as_slice());

	out.push('\n');
}

fn render_markdown_adapter_scenarios(out: &mut String, adapters: &[ExternalAdapterReport]) {
	if !has_adapter_scenarios(adapters) {
		return;
	}

	out.push_str("\n### Adapter Scenario Judgments\n\n");
	out.push_str("| Adapter | Scenario | Suite | Status | Outcome | Evidence |\n");
	out.push_str("| --- | --- | --- | --- | --- | --- |\n");

	for adapter in adapters {
		for scenario in &adapter.scenarios {
			out.push_str(&format!(
				"| `{}` | `{}` | {} | `{}` | `{}` | {} |\n",
				md_inline(adapter.adapter_id.as_str()),
				md_inline(scenario.scenario_id.as_str()),
				scenario
					.suite_id
					.as_deref()
					.map(|suite| format!("`{}`", md_inline(suite)))
					.unwrap_or_else(|| "`none`".to_string()),
				adapter_status_str(scenario.status),
				scenario_comparison_outcome_str(scenario_comparison_outcome(scenario)),
				adapter_scenario_evidence_cell(scenario)
			));
		}
	}
}

fn has_adapter_scenarios(adapters: &[ExternalAdapterReport]) -> bool {
	adapters.iter().any(|adapter| !adapter.scenarios.is_empty())
}

fn render_markdown_adapter_execution_metadata(
	out: &mut String,
	adapters: &[ExternalAdapterReport],
) {
	let mut wrote_header = false;

	for adapter in adapters {
		let Some(metadata) = &adapter.execution_metadata else {
			continue;
		};

		if !wrote_header {
			out.push_str("\n### Adapter Execution Metadata\n\n");
			out.push_str("| Adapter | Sources | Setup Path | Runtime Boundary | Resource Expectation | Retry Guidance | Research Depth |\n");
			out.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");

			wrote_header = true;
		}

		out.push_str(&format!(
			"| `{}` | {} | {} | {} | {} | {} | {} |\n",
			md_inline(adapter.adapter_id.as_str()),
			adapter_sources_cell(metadata.sources.as_slice()),
			md_cell(metadata.setup_path.as_str()),
			md_cell(metadata.runtime_boundary.as_str()),
			md_cell(metadata.resource_expectation.as_str()),
			md_list(metadata.retry_guidance.as_slice()),
			md_cell(metadata.research_depth.as_deref().unwrap_or("not recorded"))
		));
	}
}

fn render_markdown_header(out: &mut String, report: &RealWorldReport, report_path: &str) {
	out.push_str("# Real-World Job Benchmark Report\n\n");
	out.push_str(
		"Goal: Publish a Markdown summary for one generated real_world_job benchmark report.\n",
	);
	out.push_str(
		"Read this when: You need a durable smoke report for real-world agent memory job fixtures.\n",
	);
	out.push_str(&format!("Inputs: `{}`.\n", md_inline(report_path)));
	out.push_str("Depends on: `apps/elf-eval/fixtures/`, `docs/spec/real_world_agent_memory_benchmark_v1.md`, and `Makefile.toml`.\n");
	out.push_str(
		"Verification: Compare this Markdown summary with the source JSON before committing.\n\n",
	);
	out.push_str("## Summary\n\n");
	out.push_str(&format!("- Run ID: `{}`\n", md_inline(report.run_id.as_str())));
	out.push_str(&format!("- Generated at: `{}`\n", md_inline(report.generated_at.as_str())));
	out.push_str(&format!("- Runner version: `{}`\n", md_inline(report.runner_version.as_str())));
	out.push_str(&format!("- Corpus profile: `{}`\n", md_inline(report.corpus_profile.as_str())));
	out.push_str(&format!(
		"- Adapter: `{}` ({})\n",
		md_inline(report.adapter.adapter_id.as_str()),
		md_inline(report.adapter.behavior.as_str())
	));
	out.push_str(&format!("- Jobs: `{}`\n", report.summary.job_count));
	out.push_str(&format!(
		"- Suites with encoded jobs: `{}`\n",
		report.summary.encoded_suite_count
	));
	out.push_str(&format!(
		"- Suites with `not_encoded` status: `{}`\n",
		report.not_encoded_suites.len()
	));
	out.push_str(&format!("- Status summary: `{}` pass, `{}` wrong_result, `{}` lifecycle_fail, `{}` incomplete, `{}` blocked, `{}` not_encoded, `{}` unsupported_claim\n", report.summary.pass, report.summary.wrong_result, report.summary.lifecycle_fail, report.summary.incomplete, report.summary.blocked, report.summary.not_encoded, report.summary.unsupported_claim));
	out.push_str(&format!(
		"- Unsupported claim count: `{}`\n",
		report.summary.unsupported_claim_count
	));
	out.push_str(&format!("- Wrong-result count: `{}`\n", report.summary.wrong_result_count));
	out.push_str(&format!("- Stale-answer count: `{}`\n", report.summary.stale_answer_count));
	out.push_str(&format!(
		"- Conflict detections: `{}`\n",
		report.summary.conflict_detection_count
	));
	out.push_str(&format!(
		"- Update rationales available: `{}`\n",
		report.summary.update_rationale_available_count
	));
	out.push_str(&format!(
		"- Temporal validity not encoded: `{}`\n",
		report.summary.temporal_validity_not_encoded_count
	));
	out.push_str(&format!(
		"- History readback encoded: `{}`\n",
		report.summary.history_readback_encoded_count
	));

	render_markdown_quality_summary(out, report);

	out.push_str(&format!("- Mean score: `{:.3}`\n", report.summary.mean_score));
	out.push_str(&format!(
		"- Mean latency: `{}`\n",
		optional_f64(report.summary.mean_latency_ms, " ms")
	));
	out.push_str(&format!("- Cost: `{}`\n", cost_display(report.summary.total_cost.as_ref())));
	out.push_str(&format!(
		"- Operator-debug jobs: `{}`\n",
		report.summary.operator_debug_job_count
	));
	out.push_str(&format!("- Raw SQL needed: `{}`\n", report.summary.raw_sql_needed_count));
	out.push_str(&format!(
		"- Trace-incomplete debug jobs: `{}`\n",
		report.summary.trace_incomplete_count
	));
	out.push_str(&format!("- Operator UX gaps: `{}`\n", report.summary.operator_ux_gap_count));

	if let Some(knowledge) = &report.summary.knowledge {
		out.push_str(&format!(
			"- Knowledge citation coverage: `{:.3}`\n",
			knowledge.citation_coverage
		));
		out.push_str(&format!(
			"- Stale claim detection: `{:.3}`\n",
			knowledge.stale_claim_detection
		));
		out.push_str(&format!("- Rebuild determinism: `{:.3}`\n", knowledge.rebuild_determinism));
		out.push_str(&format!(
			"- Backlinks: `{}` total, `{:.3}` page coverage\n",
			knowledge.backlink_count, knowledge.backlink_coverage
		));
		out.push_str(&format!("- Page usefulness: `{:.3}`\n", knowledge.page_usefulness));
		out.push_str(&format!(
			"- Unsupported summary count: `{}`\n",
			knowledge.unsupported_summary_count
		));
	}

	out.push_str(&format!(
		"- Private corpus redaction: `{}`\n\n",
		md_inline(report.private_corpus_redaction.policy.as_str())
	));
}

fn render_markdown_quality_summary(out: &mut String, report: &RealWorldReport) {
	out.push_str(&format!(
		"- Evidence coverage: `{}/{}` (`{:.3}`)\n",
		report.summary.evidence_covered_count,
		report.summary.evidence_required_count,
		report.summary.evidence_coverage
	));
	out.push_str(&format!(
		"- Source-ref coverage: `{}/{}` (`{:.3}`)\n",
		report.summary.source_ref_covered_count,
		report.summary.source_ref_required_count,
		report.summary.source_ref_coverage
	));
	out.push_str(&format!(
		"- Quote coverage: `{}/{}` (`{:.3}`)\n",
		report.summary.quote_covered_count,
		report.summary.quote_required_count,
		report.summary.quote_coverage
	));
	out.push_str(&format!("- Stale retrieval count: `{}`\n", report.summary.stale_retrieval_count));
	out.push_str(&format!(
		"- Scope correctness: `{}/{}` (`{:.3}`), violations `{}`\n",
		report.summary.scope_correct_count,
		report.summary.scope_check_count,
		report.summary.scope_correctness,
		report.summary.scope_violation_count
	));
	out.push_str(&format!("- Redaction leak count: `{}`\n", report.summary.redaction_leak_count));
	out.push_str(&format!(
		"- Qdrant rebuild cases: `{}` encoded, `{}` pass\n",
		report.summary.qdrant_rebuild_case_count, report.summary.qdrant_rebuild_pass_count
	));
	out.push_str(&format!(
		"- Expected evidence recall: `{:.3}` ({}/{})\n",
		report.summary.expected_evidence_recall,
		report.summary.expected_evidence_matched,
		report.summary.expected_evidence_total
	));
	out.push_str(&format!(
		"- Irrelevant context ratio: `{:.3}` ({} irrelevant)\n",
		report.summary.irrelevant_context_ratio, report.summary.irrelevant_context_count
	));
	out.push_str(&format!(
		"- Trace explainability: `{}` job(s), `{}` wrong-result stage attribution(s)\n",
		report.summary.trace_explainability_count,
		report.summary.wrong_result_stage_attribution_count
	));
	out.push_str(&format!(
		"- Consolidation source mutation count: `{}`\n",
		report.summary.consolidation.source_mutation_count
	));
}

fn render_markdown_suites(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Suites\n\n");
	out.push_str(
		"| Suite | Status | Jobs | Score | Evidence Recall | Irrelevant Context | Trace Explain | Stale Answers | Conflicts | Update Rationales | Temporal Gaps | History Readback | Unsupported Claims | Wrong Results | Reason |\n",
	);
	out.push_str("| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |\n");

	for suite in &report.suites {
		out.push_str(&format!(
			"| {} | `{}` | {} | `{}` | `{}` | `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
			md_cell(suite.suite_id.as_str()),
			status_str(suite.status),
			suite.encoded_job_count,
			optional_f64(suite.score_mean, ""),
			optional_f64(suite.expected_evidence_recall, ""),
			optional_f64(suite.irrelevant_context_ratio, ""),
			suite.trace_explainability_count,
			suite.stale_answer_count,
			suite.conflict_detection_count,
			suite.update_rationale_available_count,
			suite.temporal_validity_not_encoded_count,
			suite.history_readback_encoded_count,
			suite.unsupported_claim_count,
			suite.wrong_result_count,
			md_cell(suite.reason.as_str())
		));
	}

	out.push('\n');
}

fn render_markdown_jobs(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Jobs\n\n");
	out.push_str("| Suite | Job | Status | Answer Type | Caveat Required | Refusal Required | Unknown Allowed | Score | Evidence Recall | Irrelevant Context | Expected Evidence | Produced Evidence | Trace Failure Stage | Stale Answers | Conflicts | Update Rationale | Temporal Gap | Unsupported Claims | Wrong Results | Latency | Cost |\n");
	out.push_str(
		"| --- | --- | --- | --- | --- | --- | --- | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | --- | --- | ---: | ---: | ---: | --- |\n",
	);

	for job in &report.jobs {
		let expected = job
			.expected_evidence
			.iter()
			.map(|evidence| evidence.evidence_id.as_str())
			.collect::<Vec<_>>()
			.join(", ");
		let produced = job.produced_evidence.join(", ");

		out.push_str(&format!(
			"| {} | {} | `{}` | `{}` | `{}` | `{}` | `{}` | `{:.3}` | `{:.3}` | `{:.3}` | `{}` | `{}` | `{}` | {} | {} | `{}` | `{}` | {} | {} | `{}` | `{}` |\n",
			md_cell(job.suite_id.as_str()),
			md_cell(job.job_id.as_str()),
			status_str(job.status),
			md_inline(job.answer_type.as_str()),
			bool_display(job.requires_caveat),
			bool_display(job.requires_refusal),
			bool_display(job.can_answer_unknown),
			job.normalized_score,
			job.retrieval_quality.expected_evidence_recall,
			job.retrieval_quality.irrelevant_context_ratio,
			md_inline(expected.as_str()),
			md_inline(produced.as_str()),
			md_inline(trace_failure_stage(job.trace_explainability.as_ref()).unwrap_or("-")),
			job.stale_answer_count,
			job.conflict_detection_count,
			bool_display(job.update_rationale_available),
			bool_display(job.temporal_validity_not_encoded),
			job.unsupported_claim_count,
			job.wrong_result_count,
			optional_f64(job.latency_ms, " ms"),
			cost_display(job.cost.as_ref())
		));
	}

	out.push('\n');
}

fn render_markdown_operator_debugging(out: &mut String, report: &RealWorldReport) {
	let jobs = report.jobs.iter().filter(|job| job.operator_debug.is_some()).collect::<Vec<_>>();

	out.push_str("## Operator Debugging UX\n\n");

	if jobs.is_empty() {
		out.push_str("No encoded job reported operator debugging evidence.\n\n");

		return;
	}

	out.push_str("| Job | Failure Mode | Trace Evidence | Trace Available | Replay Command | Steps | Raw SQL | Dropped Candidate Visibility | Trace Completeness | Repair Clarity | UX Gaps |\n");
	out.push_str("| --- | --- | --- | --- | --- | ---: | --- | --- | --- | --- | --- |\n");

	for job in jobs {
		if let Some(debug) = &job.operator_debug {
			out.push_str(&format!(
				"| {} | {} | {} | `{}` | `{}` | {} | `{}` | {} | `{}` | `{}` | {} |\n",
				md_cell(job.job_id.as_str()),
				md_cell(debug.failure_mode.as_str()),
				debug_trace_cell(debug),
				debug.trace_available.unwrap_or(debug.trace_id.is_some()),
				debug.replay_command_available.unwrap_or(debug.replay_command.is_some()),
				debug.steps_to_root_cause,
				debug.raw_sql_needed,
				md_cell(debug.dropped_candidate_visibility.as_str()),
				md_inline(debug.trace_completeness.as_str()),
				md_inline(debug.repair_action_clarity.as_str()),
				ux_gap_cell(debug.ux_gaps.as_slice())
			));
		}
	}

	out.push_str("\n### Operator Debug Details\n\n");

	for job in report.jobs.iter().filter(|job| job.operator_debug.is_some()) {
		if let Some(debug) = &job.operator_debug {
			out.push_str(&format!("#### `{}`\n\n", md_inline(job.job_id.as_str())));
			out.push_str(&format!("- Root cause: {}\n", md_cell(debug.root_cause.as_str())));
			out.push_str(&format!(
				"- Viewer panels: `{}`\n",
				md_inline(debug.viewer_panels.join(", ").as_str())
			));
			out.push_str(&format!(
				"- CLI steps: `{}`\n",
				md_inline(debug.cli_steps.join(" -> ").as_str())
			));

			if let Some(command) = &debug.replay_command {
				out.push_str(&format!("- Replay command: `{}`\n", md_inline(command.as_str())));
			}
			if let Some(artifact) = &debug.replay_artifact {
				out.push_str(&format!("- Replay artifact: `{}`\n", md_inline(artifact.as_str())));
			}

			out.push_str(&format!(
				"- Trace evidence: `{}`\n",
				md_inline(debug.trace_evidence.join(", ").as_str())
			));
			out.push('\n');
		}
	}
}

fn debug_trace_cell(debug: &OperatorDebugEvidence) -> String {
	let trace = debug.trace_id.as_deref().unwrap_or("-");
	let viewer = debug
		.viewer_url
		.as_deref()
		.map(|url| format!("[viewer]({})", md_url(url)))
		.unwrap_or_else(|| "viewer: -".to_string());
	let bundle = debug
		.admin_trace_bundle_url
		.as_deref()
		.map(|url| format!("[bundle]({})", md_url(url)))
		.unwrap_or_else(|| "bundle: -".to_string());

	format!("`{}`<br>{}<br>{}", md_inline(trace), viewer, bundle)
}

fn ux_gap_cell(gaps: &[OperatorUxGap]) -> String {
	if gaps.is_empty() {
		return "`none`".to_string();
	}

	gaps.iter()
		.map(|gap| {
			format!(
				"`{}`: {} ({})",
				md_inline(gap.gap_id.as_str()),
				md_cell(gap.description.as_str()),
				md_inline(gap.follow_up_issue.as_str())
			)
		})
		.collect::<Vec<_>>()
		.join("<br>")
}

fn render_markdown_evolution(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Memory Evolution\n\n");
	out.push_str(&format!("- Stale answers: `{}`\n", report.evolution.stale_answer_count));
	out.push_str(&format!(
		"- Conflict detections: `{}`\n",
		report.evolution.conflict_detection_count
	));
	out.push_str(&format!(
		"- Update rationales available: `{}`\n",
		report.evolution.update_rationale_available_count
	));
	out.push_str(&format!(
		"- Temporal validity not encoded: `{}`\n\n",
		report.evolution.temporal_validity_not_encoded_count
	));
	out.push_str(&format!(
		"- History readback encoded: `{}`\n\n",
		report.evolution.history_readback_encoded_count
	));
	out.push_str("| Suite | Job | Current Evidence | Historical Evidence | Stale Traps Used | Conflict Count | Detected | Update Rationale | Temporal Validity | History Readback | Follow-up |\n");
	out.push_str("| --- | --- | --- | --- | --- | ---: | ---: | --- | --- | --- | --- |\n");

	for job in &report.jobs {
		let Some(evolution) = &job.evolution else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | `{}` | `{}` | `{}` | {} | {} | `{}` | `{}` | `{}` | {} |\n",
			md_cell(job.suite_id.as_str()),
			md_cell(job.job_id.as_str()),
			md_inline(evolution.current_evidence.join(", ").as_str()),
			md_inline(evolution.historical_evidence.join(", ").as_str()),
			md_inline(evolution.stale_trap_ids_used.join(", ").as_str()),
			evolution.conflict_count,
			evolution.conflict_detection_count,
			bool_display(evolution.update_rationale_available),
			temporal_display(evolution),
			history_display(evolution),
			md_cell(evolution.follow_up.as_deref().unwrap_or("-"))
		));
	}

	out.push('\n');
}

fn render_markdown_trace_explainability(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Trace Explainability\n\n");

	let jobs =
		report.jobs.iter().filter(|job| job.trace_explainability.is_some()).collect::<Vec<_>>();

	if jobs.is_empty() {
		out.push_str("No encoded job reported trace explainability metadata.\n\n");

		return;
	}

	out.push_str("| Suite | Job | Trace | Failure Stage | Reason | Stage Evidence |\n");
	out.push_str("| --- | --- | --- | --- | --- | --- |\n");

	for job in jobs {
		let trace = job.trace_explainability.as_ref();

		out.push_str(&format!(
			"| {} | {} | `{}` | `{}` | {} | {} |\n",
			md_cell(job.suite_id.as_str()),
			md_cell(job.job_id.as_str()),
			md_inline(trace.and_then(|trace| trace.trace_id.as_deref()).unwrap_or("-")),
			md_inline(trace_failure_stage(trace).unwrap_or("-")),
			md_cell(trace_failure_reason(trace).unwrap_or("-")),
			md_cell(trace_stage_summary(trace).as_str())
		));
	}

	out.push('\n');
}

fn render_markdown_consolidation(out: &mut String, report: &RealWorldReport) {
	if report.summary.consolidation.proposal_count == 0 {
		return;
	}

	out.push_str("## Consolidation\n\n");
	out.push_str("| Job | Proposals | Usefulness | Lineage | Review Actions | Source Mutations | Proposal Unsupported Claims | Executable Gaps |\n");
	out.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");

	for job in &report.jobs {
		let Some(consolidation) = &job.consolidation else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | `{}` | `{}` | `{}` | {} | {} | {} |\n",
			md_cell(job.job_id.as_str()),
			consolidation.proposal_count,
			optional_f64(consolidation.proposal_usefulness, ""),
			optional_f64(consolidation.lineage_completeness, ""),
			optional_f64(consolidation.review_action_correctness, ""),
			consolidation.source_mutation_count,
			consolidation.proposal_unsupported_claim_count,
			consolidation.executable_gaps.len()
		));
	}

	out.push_str(
		"\nSource mutation count must remain `0` for proposal-only consolidation cases.\n\n",
	);

	render_markdown_consolidation_gaps(out, report);
}

fn render_markdown_consolidation_gaps(out: &mut String, report: &RealWorldReport) {
	let gaps = report
		.jobs
		.iter()
		.filter_map(|job| job.consolidation.as_ref().map(|consolidation| (job, consolidation)))
		.flat_map(|(job, consolidation)| {
			consolidation.executable_gaps.iter().map(move |gap| (job.job_id.as_str(), gap))
		})
		.collect::<Vec<_>>();

	if gaps.is_empty() {
		return;
	}

	out.push_str("### Executable Gaps\n\n");
	out.push_str("| Job | Primitive | Follow-Up Issue | Blocks Fixture Pass | Reason |\n");
	out.push_str("| --- | --- | --- | --- | --- |\n");

	for (job_id, gap) in gaps {
		out.push_str(&format!(
			"| {} | {} | {} | `{}` | {} |\n",
			md_cell(job_id),
			md_cell(gap.primitive.as_str()),
			md_cell(gap.follow_up_issue.as_str()),
			gap.blocks_fixture_pass,
			md_cell(gap.reason.as_str())
		));
	}

	out.push('\n');
}

fn render_markdown_knowledge(out: &mut String, report: &RealWorldReport) {
	let knowledge_jobs =
		report.jobs.iter().filter(|job| job.knowledge.is_some()).collect::<Vec<_>>();

	if knowledge_jobs.is_empty() {
		return;
	}

	out.push_str("## Knowledge Page Metrics\n\n");
	out.push_str("| Job | Pages | Sections | Citation Coverage | Stale Claim Detection | Rebuild Determinism | Page Usefulness | Backlinks | Unsupported Summaries | Untraced Sections | Allowed Variance |\n");
	out.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");

	for job in knowledge_jobs {
		let Some(knowledge) = &job.knowledge else {
			continue;
		};

		out.push_str(&format!(
			"| {} | {} | {} | `{:.3}` | `{:.3}` | `{:.3}` | `{:.3}` | {} | {} | {} | {} |\n",
			md_cell(job.job_id.as_str()),
			knowledge.page_count,
			knowledge.section_count,
			knowledge.citation_coverage,
			knowledge.stale_claim_detection,
			knowledge.rebuild_determinism,
			knowledge.page_usefulness,
			knowledge.backlink_count,
			knowledge.unsupported_summary_count,
			knowledge.untraced_section_count,
			knowledge.allowed_variance_count
		));
	}

	out.push('\n');
}

fn render_markdown_unsupported_claims(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Unsupported Claims\n\n");

	if report.unsupported_claims.is_empty() {
		out.push_str("No unsupported claims were produced by encoded jobs.\n\n");

		return;
	}

	out.push_str("| Suite | Job | Claim | Evidence | Reason |\n");
	out.push_str("| --- | --- | --- | --- | --- |\n");

	for claim in &report.unsupported_claims {
		out.push_str(&format!(
			"| {} | {} | {} | `{}` | {} |\n",
			md_cell(claim.suite_id.as_str()),
			md_cell(claim.job_id.as_str()),
			md_cell(claim.claim_text.as_str()),
			md_inline(claim.evidence_ids.join(", ").as_str()),
			md_cell(claim.reason.as_str())
		));
	}

	out.push('\n');
}

fn render_markdown_follow_ups(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Follow-Ups\n\n");

	if report.follow_ups.is_empty() {
		out.push_str("No benchmark follow-ups were declared by encoded jobs.\n\n");

		return;
	}

	out.push_str("| Suite | Job | Follow-up | Reason |\n");
	out.push_str("| --- | --- | --- | --- |\n");

	for follow_up in &report.follow_ups {
		out.push_str(&format!(
			"| {} | {} | {} | {} |\n",
			md_cell(follow_up.suite_id.as_str()),
			md_cell(follow_up.job_id.as_str()),
			md_cell(follow_up.title.as_str()),
			md_cell(follow_up.reason.as_str())
		));
	}

	out.push('\n');
}

fn render_markdown_semantics(out: &mut String, report: &RealWorldReport) {
	out.push_str("## Result Semantics\n\n");
	out.push_str(
		"This report uses `docs/spec/real_world_agent_memory_benchmark_v1.md` status terms.\n",
	);
	out.push_str("It is a real-world job fixture report, not a Docker live-baseline report.\n");
	out.push_str("Existing live-baseline reports remain valid for their encoded retrieval and lifecycle checks and are not reinterpreted as real-world suite wins.\n\n");
	out.push_str(
		"The summary counters report required evidence coverage, source-ref coverage, quote coverage, expected evidence recall, irrelevant context ratio, trace explainability, stale retrievals, scope violations, redaction leaks, Qdrant rebuild case coverage, stale answers, conflict detections, update rationale availability, and temporal validity gaps across encoded jobs.\n\n",
	);
	out.push_str(
		"- `pass`: encoded jobs met their pass threshold with required evidence and no hard-fail rule.\n",
	);
	out.push_str(
		"- `wrong_result`: a job completed but missed required answer or evidence expectations.\n",
	);
	out.push_str("- `unsupported_claim`: a job produced a substantive claim not supported by the fixture evidence links.\n");
	out.push_str("- `not_encoded`: a suite has no checked-in fixture, or an encoded fixture declares a capability gap so no pass/fail claim is allowed.\n\n");
	out.push_str("For `knowledge_compilation` jobs, generated pages are benchmark artifacts. Page sections must cite source evidence or timeline events, or be explicitly flagged as unsupported. Flagged unsupported summaries are counted separately from hidden unsupported claims.\n\n");
	out.push_str("## Suites With `not_encoded` Status\n\n");

	if report.not_encoded_suites.is_empty() {
		out.push_str("All declared suites have at least one encoded job.\n");
	} else {
		for suite in &report.not_encoded_suites {
			out.push_str(&format!("- `{}`\n", md_inline(suite.as_str())));
		}
	}
}

fn status_str(status: TypedStatus) -> &'static str {
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

fn adapter_status_str(status: AdapterCoverageStatus) -> &'static str {
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

fn scenario_comparison_outcome_str(outcome: ScenarioComparisonOutcome) -> &'static str {
	match outcome {
		ScenarioComparisonOutcome::Win => "win",
		ScenarioComparisonOutcome::Tie => "tie",
		ScenarioComparisonOutcome::Loss => "loss",
		ScenarioComparisonOutcome::NotTested => "not_tested",
		ScenarioComparisonOutcome::Blocked => "blocked",
		ScenarioComparisonOutcome::NonGoal => "non_goal",
	}
}

fn adapter_status_counts_display(counts: &AdapterStatusCounts) -> String {
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

fn scenario_position_counts_display(counts: &ScenarioPositionCounts) -> String {
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

fn scenario_outcome_counts_display(counts: &ScenarioOutcomeCounts) -> String {
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

fn adapter_suite_cell(suites: &[AdapterSuiteCoverage]) -> String {
	if suites.is_empty() {
		return "`none`".to_string();
	}

	suites
		.iter()
		.map(|suite| {
			format!(
				"`{}`: `{}`",
				md_inline(suite.suite_id.as_str()),
				adapter_status_str(suite.status)
			)
		})
		.collect::<Vec<_>>()
		.join("<br>")
}

fn adapter_evidence_cell(adapter: &ExternalAdapterReport) -> String {
	let setup = adapter
		.setup
		.command
		.as_deref()
		.or(adapter.setup.artifact.as_deref())
		.unwrap_or(adapter.setup.evidence.as_str());
	let result = adapter
		.result
		.artifact
		.as_deref()
		.or(adapter.result.command.as_deref())
		.unwrap_or(adapter.result.evidence.as_str());

	format!("setup: `{}`<br>result: `{}`", md_inline(setup), md_inline(result))
}

fn adapter_scenario_evidence_cell(scenario: &AdapterScenarioJudgment) -> String {
	let evidence = md_cell(scenario.evidence.as_str());
	let command = scenario
		.command
		.as_deref()
		.map(|command| format!("<br>command: `{}`", md_inline(command)))
		.unwrap_or_default();
	let artifact = scenario
		.artifact
		.as_deref()
		.map(|artifact| format!("<br>artifact: `{}`", md_inline(artifact)))
		.unwrap_or_default();

	format!("{evidence}{command}{artifact}")
}

fn adapter_sources_cell(sources: &[AdapterSource]) -> String {
	if sources.is_empty() {
		return "`none`".to_string();
	}

	sources
		.iter()
		.map(|source| {
			format!(
				"[{}]({}): {}",
				md_cell(source.label.as_str()),
				md_url(source.url.as_str()),
				md_cell(source.evidence.as_str())
			)
		})
		.collect::<Vec<_>>()
		.join("<br>")
}

fn trace_failure_stage(trace: Option<&TraceExplainability>) -> Option<&str> {
	trace.and_then(|trace| trace.failure_stage.as_deref())
}

fn trace_failure_reason(trace: Option<&TraceExplainability>) -> Option<&str> {
	trace.and_then(|trace| trace.failure_reason.as_deref())
}

fn trace_stage_summary(trace: Option<&TraceExplainability>) -> String {
	let Some(trace) = trace else {
		return "-".to_string();
	};
	let stages = trace
		.stages
		.iter()
		.map(|stage| {
			format!(
				"{} kept={} demoted={} dropped={} distractors={}",
				stage.stage_name,
				stage.kept_evidence.join("+"),
				stage.demoted_evidence.join("+"),
				stage.dropped_evidence.join("+"),
				stage.distractor_evidence.join("+")
			)
		})
		.collect::<Vec<_>>();

	if stages.is_empty() { "-".to_string() } else { stages.join("; ") }
}

fn write_or_print(path: Option<&Path>, content: &str) -> Result<()> {
	if let Some(path) = path {
		if let Some(parent) = path.parent()
			&& !parent.as_os_str().is_empty()
		{
			fs::create_dir_all(parent)?;
		}

		fs::write(path, content)?;

		println!("Wrote {}", path.display());
	} else {
		println!("{content}");
	}

	Ok(())
}

fn optional_f64(value: Option<f64>, suffix: &str) -> String {
	value.map(|value| format!("{value:.3}{suffix}")).unwrap_or_else(|| "-".to_string())
}

fn bool_display(value: bool) -> &'static str {
	if value { "true" } else { "false" }
}

fn temporal_display(evolution: &EvolutionJobReport) -> &'static str {
	if evolution.temporal_validity_not_encoded {
		"not_encoded"
	} else if evolution.temporal_validity_encoded {
		"encoded"
	} else if evolution.temporal_validity_required {
		"required"
	} else {
		"-"
	}
}

fn history_display(evolution: &EvolutionJobReport) -> String {
	if !evolution.history_readback_encoded {
		return "-".to_string();
	}

	let mut parts = vec![format!("events={}", evolution.history_event_types.join(","))];

	if evolution.history_requires_note_version_links {
		parts.push("note_version_links=true".to_string());
	}

	parts.join(";")
}

fn cost_display(cost: Option<&CostReport>) -> String {
	let Some(cost) = cost else {
		return "-".to_string();
	};

	match (cost.amount, cost.currency.as_deref()) {
		(Some(amount), Some(currency)) => format!("{amount:.3} {currency}"),
		(Some(amount), None) => format!("{amount:.3}"),
		(None, _) => "-".to_string(),
	}
}

fn bounded_text(value: &str, max_chars: usize) -> String {
	let mut chars = value.chars();
	let text = chars.by_ref().take(max_chars).collect::<String>();

	if chars.next().is_some() { format!("{text}...") } else { text }
}

fn md_inline(value: &str) -> String {
	value.replace('`', "'").replace('\n', " ")
}

fn md_cell(value: &str) -> String {
	md_inline(value).replace('|', "\\|")
}

fn md_url(value: &str) -> String {
	value.replace(')', "%29").replace(' ', "%20")
}

fn md_list(values: &[String]) -> String {
	if values.is_empty() {
		return "-".to_string();
	}

	md_cell(values.join("; ").as_str())
}

fn round3(value: f64) -> f64 {
	(value * 1_000.0).round() / 1_000.0
}
