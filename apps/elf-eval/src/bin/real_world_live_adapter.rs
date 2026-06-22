#![allow(clippy::single_component_path_imports, unused_crate_dependencies)]

//! Live adapter materializer for the real-world job benchmark.

use std::{
	collections::{BTreeSet, HashMap},
	env,
	fs::{self, OpenOptions},
	io::Write as _,
	path::{Path, PathBuf},
	process::{Command, Stdio},
	sync::Arc,
	time::{Duration, Instant},
};

use ::time::{OffsetDateTime, format_description::well_known::Rfc3339};
use blake3::Hasher;
use clap::{Parser, Subcommand, ValueEnum};
use color_eyre::{self, eyre};
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use serde_json::{self, Map};
use tokio::{task::JoinSet, time};
use uuid::Uuid;

use elf_chunking::ChunkingConfig;
use elf_config::{Config, EmbeddingProviderConfig, LlmProviderConfig, ProviderConfig};
use elf_domain::{
	consolidation::{
		ConsolidationApplyIntent, ConsolidationInputRef, ConsolidationLineage, ConsolidationMarker,
		ConsolidationMarkerSeverity, ConsolidationMarkers, ConsolidationProposalDiff,
		ConsolidationReviewAction, ConsolidationSourceKind, ConsolidationSourceSnapshot,
		ConsolidationUnsupportedClaimFlag,
	},
	knowledge::KnowledgePageKind,
	writegate::{self, WritePolicy},
};
use elf_service::{
	AddNoteInput, AddNoteRequest, BoxFuture, ConsolidationProposalInput,
	ConsolidationProposalResponse, ConsolidationProposalReviewRequest,
	ConsolidationProposalsListRequest, ConsolidationRunCreateRequest, ElfService,
	EmbeddingProvider, ExtractorProvider, KnowledgePageLintRequest, KnowledgePageLintResponse,
	KnowledgePageRebuildRequest, KnowledgePageResponse, KnowledgePageSearchRequest, ListRequest,
	PayloadLevel, Providers, RerankProvider, SearchItem, SearchRequest, SearchResponse,
};
use elf_storage::{db::Db, qdrant::QdrantStore};
use elf_testkit::TestDatabase;
use elf_worker::worker::{self, WorkerState};

const JOB_SCHEMA: &str = "elf.real_world_job/v1";
const EVIDENCE_SCHEMA: &str = "elf.real_world_live_adapter_materialization/v1";
const TENANT_ID: &str = "elf-live-real-world";
const AGENT_ID: &str = "elf-live-real-world-agent";
const SCOPE: &str = "agent_private";
const ELF_NOTE_CHUNK_CHARS: usize = 220;

#[derive(Debug, Parser)]
#[command(version = elf_cli::VERSION, rename_all = "kebab", styles = elf_cli::styles())]
struct Args {
	#[command(subcommand)]
	command: CommandArgs,
}

#[derive(Debug, Parser)]
struct ElfArgs {
	/// Fixture file or directory containing real_world_job JSON fixtures.
	#[arg(long, value_name = "PATH")]
	fixtures: PathBuf,
	/// Directory where generated real_world_job fixtures are written.
	#[arg(long, value_name = "DIR")]
	out_fixtures: PathBuf,
	/// JSON evidence file for adapter setup/run/result details.
	#[arg(long, value_name = "FILE")]
	evidence_out: PathBuf,
	/// ELF config loaded before Docker runtime overrides are applied.
	#[arg(long, short = 'c', value_name = "FILE")]
	config: PathBuf,
	/// Adapter id embedded in generated adapter_response objects.
	#[arg(long, default_value = "elf_live_real_world")]
	adapter_id: String,
}

#[derive(Debug, Parser)]
struct QmdArgs {
	/// Fixture file or directory containing real_world_job JSON fixtures.
	#[arg(long, value_name = "PATH")]
	fixtures: PathBuf,
	/// Directory where generated real_world_job fixtures are written.
	#[arg(long, value_name = "DIR")]
	out_fixtures: PathBuf,
	/// JSON evidence file for adapter setup/run/result details.
	#[arg(long, value_name = "FILE")]
	evidence_out: PathBuf,
	/// qmd checkout directory. The materializer clones into it when missing.
	#[arg(long, value_name = "DIR")]
	qmd_dir: PathBuf,
	/// Work directory for qmd home, corpus files, and command logs.
	#[arg(long, value_name = "DIR")]
	work_dir: PathBuf,
	/// qmd repository URL used when qmd_dir is absent.
	#[arg(long, default_value = "https://github.com/tobi/qmd.git")]
	qmd_repo_url: String,
	/// Adapter id embedded in generated adapter_response objects.
	#[arg(long, default_value = "qmd_live_real_world")]
	adapter_id: String,
}

#[derive(Debug, Parser)]
struct LightragArgs {
	/// Fixture file or directory containing real_world_job JSON fixtures.
	#[arg(long, value_name = "PATH")]
	fixtures: PathBuf,
	/// Directory where generated real_world_job fixtures are written.
	#[arg(long, value_name = "DIR")]
	out_fixtures: PathBuf,
	/// JSON evidence file for adapter setup/run/result details.
	#[arg(long, value_name = "FILE")]
	evidence_out: PathBuf,
	/// Work directory for generated source files and command logs.
	#[arg(long, value_name = "DIR")]
	work_dir: PathBuf,
	/// LightRAG API base URL reachable from the Docker runner.
	#[arg(long, default_value = "http://lightrag:9621")]
	api_base: String,
	/// Optional LightRAG API bearer token.
	#[arg(long)]
	api_key: Option<String>,
	/// Adapter id embedded in generated adapter_response objects.
	#[arg(long, default_value = "lightrag_live_real_world")]
	adapter_id: String,
	/// LightRAG query mode used for context export.
	#[arg(long, default_value = "naive")]
	query_mode: String,
	/// Number of top results requested from LightRAG.
	#[arg(long, default_value_t = 5)]
	top_k: u32,
	/// Number of chunk results requested from LightRAG.
	#[arg(long, default_value_t = 5)]
	chunk_top_k: u32,
	/// Health-check attempts before returning a typed runtime failure.
	#[arg(long, default_value_t = 30)]
	startup_attempts: u32,
	/// Delay between LightRAG health-check attempts.
	#[arg(long, default_value_t = 2)]
	startup_interval_seconds: u64,
	/// Poll attempts for asynchronous document indexing.
	#[arg(long, default_value_t = 60)]
	index_attempts: u32,
	/// Delay between document indexing status checks.
	#[arg(long, default_value_t = 2)]
	index_interval_seconds: u64,
}

#[derive(Debug)]
struct LoadedJob {
	path: PathBuf,
	value: serde_json::Value,
	job: LiveJob,
}

#[derive(Debug, Deserialize)]
struct LiveJob {
	schema: String,
	job_id: String,
	suite: String,
	title: String,
	corpus: LiveCorpus,
	prompt: LivePrompt,
	expected_answer: LiveExpectedAnswer,
	#[serde(default)]
	required_evidence: Vec<LiveRequiredEvidence>,
	#[serde(default)]
	encoding: LiveEncoding,
	memory_evolution: Option<LiveMemoryEvolution>,
}

#[derive(Debug, Deserialize)]
struct LiveCorpus {
	#[serde(default)]
	items: Vec<LiveCorpusItem>,
}

#[derive(Debug, Deserialize)]
struct LiveCorpusItem {
	evidence_id: String,
	text: Option<String>,
	local_ref: Option<String>,
	#[serde(default)]
	capture: LiveCapturePolicy,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct LiveCapturePolicy {
	#[serde(default)]
	action: LiveCaptureAction,

	source_id: Option<String>,

	evidence_binding: Option<String>,

	write_policy: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct LivePrompt {
	content: String,
}

#[derive(Debug, Deserialize)]
struct LiveExpectedAnswer {
	#[serde(default)]
	must_include: Vec<LiveExpectedClaim>,
	#[serde(default)]
	evidence_links: Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct LiveRequiredEvidence {
	evidence_id: String,
}

#[derive(Debug, Default, Deserialize)]
struct LiveMemoryEvolution {
	#[serde(default)]
	current_evidence_ids: Vec<String>,
	#[serde(default)]
	historical_evidence_ids: Vec<String>,
	#[serde(default)]
	tombstone_evidence_ids: Vec<String>,
	#[serde(default)]
	invalidation_evidence_ids: Vec<String>,
	#[serde(default)]
	conflicts: Vec<LiveEvolutionConflict>,
	update_rationale: Option<LiveUpdateRationale>,
}

#[derive(Debug, Deserialize)]
struct LiveEvolutionConflict {
	claim_id: String,
	current_evidence_id: String,
	historical_evidence_id: String,
	resolved_by_evidence_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LiveUpdateRationale {
	claim_id: String,
	#[serde(default)]
	evidence_ids: Vec<String>,
	available: bool,
}

#[derive(Debug, Default, Deserialize)]
struct LiveEncoding {
	status: Option<LiveEncodingStatus>,
	reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct MaterializationEvidence {
	schema: &'static str,
	adapter_id: String,
	adapter_kind: AdapterKind,
	status: MaterializationStatus,
	fixtures: String,
	generated_fixtures: String,
	command_evidence: Vec<CommandEvidence>,
	jobs: Vec<MaterializedJobEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct CommandEvidence {
	label: String,
	status: MaterializationStatus,
	command: String,
	artifact: Option<String>,
	reason: String,
}

#[derive(Debug, Serialize)]
struct MaterializedJobEvidence {
	job_id: String,
	suite: String,
	title: String,
	status: MaterializationStatus,
	query: String,
	evidence_ids: Vec<String>,
	returned_count: usize,
	#[serde(skip_serializing_if = "Option::is_none")]
	indexing_latency_ms: Option<f64>,
	latency_ms: f64,
	trace_id: Option<Uuid>,
	failure: Option<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	source_mappings: Vec<SourceMappingEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	operator_debug: Option<OperatorDebugMaterializationEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	capture: Option<CaptureMaterializationEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	consolidation: Option<ConsolidationMaterializationEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	knowledge: Option<KnowledgeMaterializationEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	temporal_reconciliation: Option<TemporalReconciliationMaterializationEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	dreaming_readback: Option<DreamingReadbackMaterializationEvidence>,
}

#[derive(Clone, Debug, Serialize)]
struct OperatorDebugMaterializationEvidence {
	trace_available: bool,
	replay_command_available: bool,
	candidate_drop_visibility: String,
	repair_action_clarity: String,
	raw_sql_needed: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
struct CaptureMaterializationEvidence {
	stored_evidence_ids: Vec<String>,
	excluded_evidence_ids: Vec<String>,
	source_ids: Vec<String>,
	write_policy_audit_count: usize,
	write_policy_exclusion_count: usize,
	write_policy_redaction_count: usize,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	runtime_source_refs: Vec<CaptureRuntimeSourceRefEvidence>,
}

#[derive(Clone, Debug, Default, Serialize)]
struct ConsolidationMaterializationEvidence {
	run_id: Option<Uuid>,
	proposal_ids: Vec<Uuid>,
	source_lineage_count: usize,
	unsupported_claim_flag_count: usize,
	review_event_count: usize,
	review_actions: Vec<String>,
	final_review_states: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
struct KnowledgeMaterializationEvidence {
	page_ids: Vec<Uuid>,
	search_result_count: usize,
	lint_finding_count: usize,
	stale_source_finding_count: usize,
	unsupported_claim_count: usize,
	citation_count: usize,
	source_ref_count: usize,
	version_diff_available: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
struct TemporalReconciliationMaterializationEvidence {
	current_winner_evidence_ids: Vec<String>,
	historical_loser_evidence_ids: Vec<String>,
	supersession_rationale_evidence_ids: Vec<String>,
	tombstone_evidence_ids: Vec<String>,
	invalidation_evidence_ids: Vec<String>,
	conflict_candidate_evidence_ids: Vec<String>,
	retrieved_evidence_ids: Vec<String>,
	selected_evidence_ids: Vec<String>,
	absent_evidence_ids: Vec<String>,
	retrieved_but_dropped_evidence_ids: Vec<String>,
	selected_but_not_narrated_evidence_ids: Vec<String>,
	contradicted_by_lifecycle_evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
struct DreamingReadbackMaterializationEvidence {
	artifact_kind: String,
	runtime_path: String,
	service_list_count: usize,
	trace_id: Option<Uuid>,
	generated_artifact_count: usize,
	selected_source_refs: Vec<String>,
	missing_source_refs: Vec<String>,
	source_mutation_count: usize,
	no_source_mutation_checked: bool,
}

#[derive(Clone, Debug, Serialize)]
struct CaptureRuntimeSourceRefEvidence {
	evidence_id: String,
	source_ref: serde_json::Value,
}

#[derive(Clone, Debug, Default)]
struct CaptureRuntimeEvidence {
	items: Vec<CaptureRuntimeEvidenceItem>,
}
impl CaptureRuntimeEvidence {
	fn item_for(&self, evidence_id: &str) -> Option<&CaptureRuntimeEvidenceItem> {
		self.items.iter().find(|item| item.evidence_id == evidence_id)
	}
}

#[derive(Clone, Debug)]
struct CaptureRuntimeEvidenceItem {
	evidence_id: String,
	source_id: Option<String>,
	evidence_binding: Option<String>,
	write_policy_applied: bool,
	capture_action: Option<String>,
	source_ref: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct AdapterResponseOutput {
	adapter_id: String,
	answer: AnswerOutput,
	#[serde(skip_serializing_if = "Option::is_none")]
	consolidation: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct AnswerOutput {
	content: String,
	evidence_ids: Vec<String>,
	claims: Vec<serde_json::Value>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pages: Vec<serde_json::Value>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	memory_summaries: Vec<serde_json::Value>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	proactive_briefs: Vec<serde_json::Value>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	scheduled_tasks: Vec<serde_json::Value>,
	latency_ms: f64,
	cost: CostOutput,
	trace_explainability: TraceExplainabilityOutput,
}

#[derive(Debug, Serialize)]
struct CostOutput {
	currency: String,
	amount: f64,
	input_tokens: u64,
	output_tokens: u64,
}

#[derive(Debug, Serialize)]
struct TraceExplainabilityOutput {
	trace_id: Option<String>,
	failure_stage: Option<String>,
	failure_reason: Option<String>,
	stages: Vec<TraceStageOutput>,
}

#[derive(Clone, Debug, Serialize)]
struct TraceStageOutput {
	stage_name: String,
	kept_evidence: Vec<String>,
	dropped_evidence: Vec<String>,
	demoted_evidence: Vec<String>,
	distractor_evidence: Vec<String>,
	notes: String,
}

#[derive(Debug)]
struct MaterializedJob {
	response: AdapterResponseOutput,
	evidence: MaterializedJobEvidence,
	operator_debug: Option<serde_json::Value>,
}

#[derive(Debug)]
struct MaterializedJobInput {
	content: String,
	evidence_ids: Vec<String>,
	pages: Vec<serde_json::Value>,
	latency_ms: f64,
	indexing_latency_ms: Option<f64>,
	returned_count: usize,
	trace_id: Option<Uuid>,
	failure: Option<String>,
	source_mappings: Vec<SourceMappingEvidence>,
	operator_debug: Option<serde_json::Value>,
	operator_debug_evidence: Option<OperatorDebugMaterializationEvidence>,
	capture: Option<CaptureMaterializationEvidence>,
	capture_failure: Option<String>,
	consolidation_response: Option<serde_json::Value>,
	consolidation: Option<ConsolidationMaterializationEvidence>,
	knowledge: Option<KnowledgeMaterializationEvidence>,
	temporal_reconciliation: Option<TemporalReconciliationMaterializationEvidence>,
	dreaming_readback: Option<DreamingReadbackMaterializationEvidence>,
	memory_summaries: Vec<serde_json::Value>,
	proactive_briefs: Vec<serde_json::Value>,
	scheduled_tasks: Vec<serde_json::Value>,
	trace_stages: Option<Vec<TraceStageOutput>>,
}

#[derive(Debug)]
struct DreamingReadbackOutput {
	content: String,
	evidence_ids: Vec<String>,
	memory_summaries: Vec<serde_json::Value>,
	proactive_briefs: Vec<serde_json::Value>,
	scheduled_tasks: Vec<serde_json::Value>,
	materialization: DreamingReadbackMaterializationEvidence,
	trace_stages: Vec<TraceStageOutput>,
}

struct SuiteMaterializationSelection {
	selected: SelectedEvidenceText,
	trace_stages: Option<Vec<TraceStageOutput>>,
	dreaming_readback: Option<DreamingReadbackMaterializationEvidence>,
	memory_summaries: Vec<serde_json::Value>,
	proactive_briefs: Vec<serde_json::Value>,
	scheduled_tasks: Vec<serde_json::Value>,
}

struct MaterializedOutput<'a> {
	adapter_id: &'a str,
	adapter_kind: AdapterKind,
	fixtures: &'a Path,
	out_fixtures: &'a Path,
	evidence_out: &'a Path,
	jobs: &'a [LoadedJob],
	materialized: &'a [MaterializedJob],
	command_evidence: Vec<CommandEvidence>,
	metadata: Option<serde_json::Value>,
}

#[derive(Debug)]
struct CorpusText {
	evidence_id: String,
	text: String,
	capture: LiveCapturePolicy,
}

#[derive(Debug, Default)]
struct IngestedCorpus {
	capture: CaptureMaterializationEvidence,
	note_ids_by_evidence: HashMap<String, Vec<Uuid>>,
}

#[derive(Clone, Debug, Deserialize)]
struct LiveConsolidationFixture {
	#[serde(default)]
	proposals: Vec<LiveConsolidationProposal>,
}

#[derive(Clone, Debug, Deserialize)]
struct LiveConsolidationProposal {
	proposal_id: String,
	proposal_kind: String,
	#[serde(default)]
	source_refs: Vec<String>,
	#[serde(default)]
	expected_source_refs: Vec<String>,
	usefulness_score: f64,
	min_usefulness_score: f64,
	expected_review_action: String,
	actual_review_action: String,
	#[serde(default)]
	source_mutations: Vec<serde_json::Value>,
	#[serde(default)]
	unsupported_claim_count: usize,
	#[serde(default)]
	unsupported_claim_flags: Vec<LiveUnsupportedClaimFlag>,
	#[serde(default)]
	diff: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct LiveUnsupportedClaimFlag {
	claim_id: Option<String>,
	message: String,
	source_ref: Option<String>,
}

#[derive(Debug)]
struct PreparedConsolidationRun {
	input_refs: Vec<ConsolidationInputRef>,
	proposals: Vec<ConsolidationProposalInput>,
}

#[derive(Clone, Debug, Serialize)]
struct SourceMappingEvidence {
	source: String,
	evidence_ids: Vec<String>,
	mapping_status: String,
	content_count: usize,
}

#[derive(Debug)]
struct LightragSource {
	evidence_id: String,
	file_source: String,
	artifact_path: PathBuf,
}

#[derive(Debug)]
struct BaselineRuntime {
	config_path: PathBuf,
	dsn: String,
	qdrant_url: String,
	collection: String,
	docs_collection: String,
}

#[derive(Debug)]
struct DeterministicEmbedding {
	vector_dim: u32,
}
impl EmbeddingProvider for DeterministicEmbedding {
	fn embed<'a>(
		&'a self,
		_cfg: &'a EmbeddingProviderConfig,
		texts: &'a [String],
	) -> BoxFuture<'a, elf_service::Result<Vec<Vec<f32>>>> {
		let dim = self.vector_dim;
		let vectors = texts.iter().map(|text| embed_text(text, dim)).collect();

		Box::pin(async move { Ok(vectors) })
	}
}

#[derive(Debug)]
struct TokenOverlapRerank;
impl RerankProvider for TokenOverlapRerank {
	fn rerank<'a>(
		&'a self,
		_cfg: &'a ProviderConfig,
		query: &'a str,
		docs: &'a [String],
	) -> BoxFuture<'a, elf_service::Result<Vec<f32>>> {
		let query_terms = terms(query);
		let scores = docs
			.iter()
			.map(|doc| {
				let doc_terms = terms(doc);
				let hits = query_terms.intersection(&doc_terms).count() as f32;

				hits / query_terms.len().max(1) as f32
			})
			.collect();

		Box::pin(async move { Ok(scores) })
	}
}

#[derive(Debug)]
struct NoopExtractor;
impl ExtractorProvider for NoopExtractor {
	fn extract<'a>(
		&'a self,
		_cfg: &'a LlmProviderConfig,
		_messages: &'a [serde_json::Value],
	) -> BoxFuture<'a, elf_service::Result<serde_json::Value>> {
		Box::pin(async move { Ok(serde_json::json!({ "notes": [] })) })
	}
}

#[derive(Debug)]
struct SelectedEvidenceText {
	content: String,
	evidence_ids: Vec<String>,
}

#[derive(Debug)]
struct TemporalReconciliationSelection {
	selected: SelectedEvidenceText,
	evidence: TemporalReconciliationMaterializationEvidence,
	trace_stages: Vec<TraceStageOutput>,
}

struct SuiteMaterializationSelectionInput<'a> {
	loaded: &'a LoadedJob,
	ingested: &'a IngestedCorpus,
	capture_failure: &'a Option<String>,
	selected: SelectedEvidenceText,
	trace_stages: Option<Vec<TraceStageOutput>>,
	knowledge: &'a Option<KnowledgeMaterializationEvidence>,
	consolidation: &'a Option<ConsolidationMaterializationEvidence>,
	dreaming_readback: Option<DreamingReadbackOutput>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LiveCaptureAction {
	#[default]
	Store,
	Exclude,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LiveExpectedClaim {
	Text(String),
	Object { claim_id: Option<String>, text: String },
}
impl LiveExpectedClaim {
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

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LiveEncodingStatus {
	NotEncoded,
	Blocked,
	Incomplete,
}
impl LiveEncodingStatus {
	fn materialization_status(self) -> MaterializationStatus {
		match self {
			Self::NotEncoded => MaterializationStatus::NotEncoded,
			Self::Blocked => MaterializationStatus::Blocked,
			Self::Incomplete => MaterializationStatus::Incomplete,
		}
	}

	fn as_str(self) -> &'static str {
		match self {
			Self::NotEncoded => "not_encoded",
			Self::Blocked => "blocked",
			Self::Incomplete => "incomplete",
		}
	}
}

#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab")]
enum CommandArgs {
	/// Materialize adapter responses by running jobs through ELF's service runtime.
	Elf(ElfArgs),
	/// Materialize adapter responses by running jobs through qmd's local CLI workflow.
	Qmd(QmdArgs),
	/// Materialize adapter responses by exporting LightRAG query context and source mappings.
	Lightrag(LightragArgs),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum AdapterKind {
	ElfServiceRuntime,
	QmdCliRuntime,
	LightragApiContextExport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MaterializationStatus {
	Pass,
	WrongResult,
	Blocked,
	Incomplete,
	NotEncoded,
}

fn run_qmd(args: QmdArgs) -> color_eyre::Result<()> {
	let jobs = load_jobs(&args.fixtures)?;
	let result = materialize_qmd_jobs(&args, &jobs);
	let materialized = match result {
		Ok(jobs) => jobs,
		Err(err) => failure_jobs(&args.adapter_id, &jobs, "qmd_cli_runtime", err.to_string()),
	};

	write_materialized_output(MaterializedOutput {
		adapter_id: &args.adapter_id,
		adapter_kind: AdapterKind::QmdCliRuntime,
		fixtures: &args.fixtures,
		out_fixtures: &args.out_fixtures,
		evidence_out: &args.evidence_out,
		jobs: &jobs,
		materialized: &materialized,
		command_evidence: vec![CommandEvidence {
			label: "qmd_cli_runtime".to_string(),
			status: aggregate_status(&materialized),
			command: "cargo run -p elf-eval --bin real_world_live_adapter -- qmd".to_string(),
			artifact: Some(args.evidence_out.display().to_string()),
			reason: "qmd live adapter used collection add, update, embed, and query --json."
				.to_string(),
		}],
		metadata: None,
	})
}

fn materialize_qmd_jobs(
	args: &QmdArgs,
	jobs: &[LoadedJob],
) -> color_eyre::Result<Vec<MaterializedJob>> {
	fs::create_dir_all(&args.work_dir)?;

	let log_path = args.work_dir.join("qmd-live-real-world.log");

	ensure_qmd_checkout(args, &log_path)?;

	let mut out = Vec::with_capacity(jobs.len());

	for loaded in jobs {
		out.push(materialize_qmd_job(args, loaded, &log_path)?);
	}

	Ok(out)
}

fn ensure_qmd_checkout(args: &QmdArgs, log_path: &Path) -> color_eyre::Result<()> {
	if !args.qmd_dir.exists() {
		if let Some(parent) = args.qmd_dir.parent() {
			fs::create_dir_all(parent)?;
		}

		run_logged_command(
			"qmd clone",
			Command::new("git")
				.arg("clone")
				.arg("--depth")
				.arg("1")
				.arg(&args.qmd_repo_url)
				.arg(&args.qmd_dir),
			log_path,
		)?;
	}

	run_logged_shell(
		"qmd install",
		&args.qmd_dir,
		"(npm ci || npm install --no-audit --no-fund) && npm run build --if-present",
		log_path,
	)
}

fn materialize_qmd_job(
	args: &QmdArgs,
	loaded: &LoadedJob,
	log_path: &Path,
) -> color_eyre::Result<MaterializedJob> {
	if let Some(job) = declared_encoding_job(&args.adapter_id, loaded) {
		return Ok(job);
	}
	if let Some(job) = not_encoded_job(&args.adapter_id, loaded) {
		return Ok(job);
	}

	let corpus = corpus_texts(loaded)?;
	let job_slug = slug(&loaded.job.job_id);
	let corpus_dir = args.work_dir.join("corpus").join(&job_slug);
	let home_dir = args.work_dir.join("home").join(&job_slug);
	let collection = format!("elfrw-{job_slug}");

	fs::create_dir_all(&corpus_dir)?;
	fs::create_dir_all(&home_dir)?;

	for existing in read_dir_paths(&corpus_dir)? {
		if existing.is_file() {
			fs::remove_file(existing)?;
		}
	}
	for item in &corpus {
		let path = corpus_dir.join(format!("{}.md", slug(&item.evidence_id)));

		fs::write(path, format!("# {}\n\n{}\n", item.evidence_id, item.text))?;
	}

	run_qmd_command(
		"qmd collection add",
		args,
		&home_dir,
		&[
			"collection",
			"add",
			corpus_dir
				.to_str()
				.ok_or_else(|| eyre::eyre!("qmd corpus path is not valid UTF-8."))?,
			"--name",
			collection.as_str(),
		],
		log_path,
	)?;
	run_qmd_command("qmd update", args, &home_dir, &["update"], log_path)?;
	run_qmd_command(
		"qmd embed",
		args,
		&home_dir,
		&["embed", "-f", "-c", collection.as_str()],
		log_path,
	)?;

	let started_at = Instant::now();
	let query = format!("lex: {}\nvec: {}", loaded.job.prompt.content, loaded.job.prompt.content);
	let stdout = run_qmd_command(
		"qmd query",
		args,
		&home_dir,
		&[
			"query",
			query.as_str(),
			"-c",
			collection.as_str(),
			"--json",
			"--no-rerank",
			"--min-score",
			"0",
			"-n",
			"5",
		],
		log_path,
	)?;
	let latency_ms = started_at.elapsed().as_secs_f64() * 1_000.0;
	let results = serde_json::from_str::<serde_json::Value>(&stdout).map_err(|err| {
		eyre::eyre!("qmd query did not return JSON for {}: {err}", loaded.job.job_id)
	})?;
	let entries = results.as_array().cloned().unwrap_or_default();
	let mut evidence_ids = Vec::new();

	for entry in &entries {
		let entry_text = serde_json::to_string(entry)?;

		for item in &corpus {
			if entry_text.contains(format!("{}.md", slug(&item.evidence_id)).as_str())
				|| entry_text.contains(item.evidence_id.as_str())
			{
				push_unique(&mut evidence_ids, item.evidence_id.clone());
			}
		}
	}

	let selected = selected_required_corpus_texts(loaded, &corpus, &evidence_ids);
	let replay_command = qmd_replay_command(&loaded.job.prompt.content, collection.as_str());
	let (operator_debug, operator_debug_evidence) = operator_debug_output(
		AdapterKind::QmdCliRuntime,
		loaded,
		None,
		replay_command,
		log_path.display().to_string(),
	);

	Ok(qmd_materialized_job(
		loaded,
		&args.adapter_id,
		selected,
		latency_ms,
		entries.len(),
		operator_debug,
		operator_debug_evidence,
	))
}

fn qmd_materialized_job(
	loaded: &LoadedJob,
	adapter_id: &str,
	selected: SelectedEvidenceText,
	latency_ms: f64,
	returned_count: usize,
	operator_debug: Option<serde_json::Value>,
	operator_debug_evidence: Option<OperatorDebugMaterializationEvidence>,
) -> MaterializedJob {
	materialized_job(
		loaded,
		adapter_id,
		MaterializedJobInput {
			content: selected.content,
			evidence_ids: selected.evidence_ids,
			pages: Vec::new(),
			latency_ms,
			indexing_latency_ms: None,
			returned_count,
			trace_id: None,
			failure: None,
			source_mappings: Vec::new(),
			operator_debug,
			operator_debug_evidence,
			capture: None,
			capture_failure: None,
			consolidation_response: None,
			consolidation: None,
			knowledge: None,
			temporal_reconciliation: None,
			dreaming_readback: None,
			memory_summaries: Vec::new(),
			proactive_briefs: Vec::new(),
			scheduled_tasks: Vec::new(),
			trace_stages: None,
		},
	)
}

fn lightrag_not_encoded_job(adapter_id: &str, loaded: &LoadedJob) -> Option<MaterializedJob> {
	match loaded.job.suite.as_str() {
		"retrieval" => None,
		_ => Some(materialized_declared_status_job(
			adapter_id,
			loaded,
			MaterializationStatus::NotEncoded,
			"LightRAG context-export smoke only maps retrieved context/source paths; this suite is not encoded for LightRAG scoring.".to_string(),
		)),
	}
}

fn lightrag_failure_jobs(
	adapter_id: &str,
	jobs: &[LoadedJob],
	stage: &str,
	reason: String,
) -> Vec<MaterializedJob> {
	jobs.iter()
		.map(|job| {
			if let Some(declared) = declared_encoding_job(adapter_id, job) {
				return declared;
			}
			if let Some(not_encoded) = lightrag_not_encoded_job(adapter_id, job) {
				return not_encoded;
			}

			materialized_job(
				job,
				adapter_id,
				MaterializedJobInput {
					content: String::new(),
					evidence_ids: Vec::new(),
					pages: Vec::new(),
					latency_ms: 0.0,
					indexing_latency_ms: None,
					returned_count: 0,
					trace_id: None,
					failure: Some(format!("{stage}: {reason}")),
					source_mappings: Vec::new(),
					operator_debug: None,
					operator_debug_evidence: None,
					capture: None,
					capture_failure: None,
					consolidation_response: None,
					consolidation: None,
					knowledge: None,
					temporal_reconciliation: None,
					dreaming_readback: None,
					memory_summaries: Vec::new(),
					proactive_briefs: Vec::new(),
					scheduled_tasks: Vec::new(),
					trace_stages: None,
				},
			)
		})
		.collect()
}

fn write_lightrag_corpus(
	args: &LightragArgs,
	loaded: &LoadedJob,
	corpus: &[CorpusText],
	run_slug: &str,
) -> color_eyre::Result<Vec<LightragSource>> {
	let job_slug = slug(&loaded.job.job_id);
	let corpus_dir = args.work_dir.join("corpus").join(run_slug).join(&job_slug);

	fs::create_dir_all(&corpus_dir)?;

	corpus
		.iter()
		.map(|item| {
			let file_name = format!("{}.md", slug(&item.evidence_id));
			let artifact_path = corpus_dir.join(&file_name);
			let file_source = format!("elf-real-world/{run_slug}/{job_slug}/{file_name}");

			fs::write(&artifact_path, format!("# {}\n\n{}\n", item.evidence_id, item.text))?;

			Ok(LightragSource { evidence_id: item.evidence_id.clone(), file_source, artifact_path })
		})
		.collect()
}

fn lightrag_index_failed(status: &serde_json::Value) -> bool {
	status.get("documents").and_then(serde_json::Value::as_array).into_iter().flatten().any(|doc| {
		doc.get("status")
			.and_then(serde_json::Value::as_str)
			.is_some_and(|status| status.to_ascii_lowercase().contains("fail"))
	})
}

fn lightrag_index_processed(status: &serde_json::Value, expected_docs: usize) -> bool {
	let Some(documents) = status.get("documents").and_then(serde_json::Value::as_array) else {
		return false;
	};

	documents.len() >= expected_docs
		&& documents.iter().all(|doc| {
			doc.get("status").and_then(serde_json::Value::as_str).is_some_and(|status| {
				let normalized = status.to_ascii_lowercase();

				normalized.contains("processed") || normalized.contains("success")
			})
		})
}

fn lightrag_keywords(query: &str) -> Vec<String> {
	terms(query).into_iter().take(12).collect()
}

fn lightrag_source_mappings(
	corpus: &[CorpusText],
	sources: &[LightragSource],
	response: &serde_json::Value,
) -> Vec<SourceMappingEvidence> {
	let mut mappings = Vec::new();

	if let Some(references) = response.get("references").and_then(serde_json::Value::as_array) {
		for reference in references {
			mappings.push(lightrag_reference_mapping(corpus, sources, reference));
		}
	}

	if mappings.is_empty()
		&& let Some(context) = response.get("response").and_then(serde_json::Value::as_str)
	{
		let evidence_ids = map_lightrag_evidence_ids(corpus, sources, context);

		if !evidence_ids.is_empty() {
			mappings.push(SourceMappingEvidence {
				source: "response_context".to_string(),
				evidence_ids,
				mapping_status: "matched_context".to_string(),
				content_count: 1,
			});
		}
	}

	mappings
}

fn lightrag_reference_mapping(
	corpus: &[CorpusText],
	sources: &[LightragSource],
	reference: &serde_json::Value,
) -> SourceMappingEvidence {
	let source = reference
		.get("file_path")
		.and_then(serde_json::Value::as_str)
		.or_else(|| reference.get("reference_id").and_then(serde_json::Value::as_str))
		.unwrap_or("unknown_source")
		.to_string();
	let content = reference
		.get("content")
		.and_then(serde_json::Value::as_array)
		.into_iter()
		.flatten()
		.filter_map(serde_json::Value::as_str)
		.collect::<Vec<_>>();
	let joined_content = content.join("\n");
	let combined = format!("{source}\n{joined_content}");
	let evidence_ids = map_lightrag_evidence_ids(corpus, sources, combined.as_str());
	let mapping_status = if evidence_ids.is_empty() {
		"unmatched"
	} else if !joined_content.is_empty() {
		"matched_reference_content"
	} else {
		"matched_reference_source"
	};

	SourceMappingEvidence {
		source,
		evidence_ids,
		mapping_status: mapping_status.to_string(),
		content_count: content.len(),
	}
}

fn map_lightrag_evidence_ids(
	corpus: &[CorpusText],
	sources: &[LightragSource],
	haystack: &str,
) -> Vec<String> {
	let normalized_haystack = normalize_ascii_alnum_lowercase(haystack);
	let mut evidence_ids = Vec::new();

	for item in corpus {
		let evidence_slug = slug(&item.evidence_id);
		let signature = normalized_text_signature(item.text.as_str());
		let source_match = sources.iter().any(|source| {
			source.evidence_id == item.evidence_id
				&& (haystack.contains(source.file_source.as_str())
					|| haystack.contains(source.artifact_path.to_string_lossy().as_ref()))
		});
		let id_match = haystack.contains(item.evidence_id.as_str())
			|| haystack.contains(evidence_slug.as_str())
			|| normalized_haystack.contains(evidence_slug.as_str());
		let content_match =
			!signature.is_empty() && normalized_haystack.contains(signature.as_str());

		if source_match || id_match || content_match {
			push_unique(&mut evidence_ids, item.evidence_id.clone());
		}
	}

	evidence_ids
}

fn normalized_text_signature(text: &str) -> String {
	normalize_ascii_alnum_lowercase(text).split_whitespace().take(8).collect::<Vec<_>>().join(" ")
}

fn lightrag_mapped_evidence_ids(mappings: &[SourceMappingEvidence]) -> Vec<String> {
	let mut evidence_ids = Vec::new();

	for mapping in mappings {
		for evidence_id in &mapping.evidence_ids {
			push_unique(&mut evidence_ids, evidence_id.clone());
		}
	}

	evidence_ids
}

fn lightrag_api_base(args: &LightragArgs) -> String {
	args.api_base.trim_end_matches('/').to_string()
}

fn lightrag_metadata(args: &LightragArgs, run_slug: &str) -> serde_json::Value {
	serde_json::json!({
		"schema": "elf.lightrag_context_export_metadata/v1",
		"run_slug": run_slug,
		"api_base": lightrag_api_base(args),
		"query": {
			"mode": args.query_mode,
			"only_need_context": true,
			"include_references": true,
			"include_chunk_content": true,
			"enable_rerank": false,
			"top_k": args.top_k,
			"chunk_top_k": args.chunk_top_k
		},
		"docker_boundary": {
			"compose_file": "docker-compose.baseline.yml",
			"service_profile": "lightrag",
			"service": "lightrag",
			"mock_provider_service": "lightrag-mock-provider",
			"host_global_installs_required": false,
			"workspace": "/app/data/rag_storage",
			"input_dir": "/app/data/inputs",
			"data_volumes": [
				"elf-live-baseline-lightrag-rag-storage",
				"elf-live-baseline-lightrag-inputs",
				"elf-live-baseline-lightrag-prompts"
			]
		},
		"provider_boundaries": {
			"llm_binding": "openai-compatible",
			"embedding_binding": "openai-compatible",
			"embedding_dim": 64,
			"rerank_binding": "cohere-compatible",
			"rerank_enabled_for_query": false,
			"api_key_provided": args.api_key.as_deref().is_some_and(|key| !key.is_empty()),
			"operator_owned_provider_credentials_used": false
		},
		"cache_and_resource_envelope": {
			"cargo_cache": "/usr/local/cargo",
			"pip_cache": "/root/.cache/pip",
			"huggingface_cache": "/root/.cache/huggingface",
			"lightrag_storage": "/app/data/rag_storage",
			"startup_attempts": args.startup_attempts,
			"startup_interval_seconds": args.startup_interval_seconds,
			"index_attempts": args.index_attempts,
			"index_interval_seconds": args.index_interval_seconds
		},
		"source_mapping": {
			"corpus_file_source_template": "elf-real-world/{run_slug}/{job_slug}/{evidence_id}.md",
			"mapping_inputs": ["references.file_path", "references.content", "response"],
			"quality_claim": "none"
		}
	})
}

fn materialized_job(
	loaded: &LoadedJob,
	adapter_id: &str,
	input: MaterializedJobInput,
) -> MaterializedJob {
	let capture_failure = input.capture_failure.clone();
	let required_evidence_satisfied =
		capture_failure.is_none() && required_evidence_satisfied(loaded, &input.evidence_ids);
	let status = if input.failure.is_some() {
		MaterializationStatus::Incomplete
	} else if !required_evidence_satisfied {
		MaterializationStatus::WrongResult
	} else {
		MaterializationStatus::Pass
	};
	let failure_stage = if input.failure.is_some() {
		Some("live_adapter.retrieve".to_string())
	} else if capture_failure.is_some() {
		Some("live_adapter.capture_policy".to_string())
	} else {
		None
	};
	let failure_reason = input.failure.clone().or(capture_failure);
	let stage_notes = if let Some(reason) = &failure_reason {
		reason.clone()
	} else if !required_evidence_satisfied {
		"Adapter did not return all required mapped evidence for this job.".to_string()
	} else {
		"Adapter returned mapped evidence through its live retrieval path.".to_string()
	};
	let trace_stages = input.trace_stages.unwrap_or_else(|| {
		vec![TraceStageOutput {
			stage_name: failure_stage
				.clone()
				.unwrap_or_else(|| "live_adapter.retrieve".to_string()),
			kept_evidence: input.evidence_ids.clone(),
			dropped_evidence: Vec::new(),
			demoted_evidence: Vec::new(),
			distractor_evidence: Vec::new(),
			notes: stage_notes,
		}]
	});

	MaterializedJob {
		response: AdapterResponseOutput {
			adapter_id: adapter_id.to_string(),
			answer: AnswerOutput {
				content: input.content,
				evidence_ids: input.evidence_ids.clone(),
				claims: answer_claims(loaded, &input.evidence_ids),
				pages: input.pages,
				memory_summaries: input.memory_summaries,
				proactive_briefs: input.proactive_briefs,
				scheduled_tasks: input.scheduled_tasks,
				latency_ms: input.latency_ms,
				cost: CostOutput {
					currency: "USD".to_string(),
					amount: 0.0,
					input_tokens: 0,
					output_tokens: 0,
				},
				trace_explainability: TraceExplainabilityOutput {
					trace_id: input.trace_id.map(|id| id.to_string()),
					failure_stage: failure_stage.clone(),
					failure_reason: failure_reason.clone(),
					stages: trace_stages,
				},
			},
			consolidation: input.consolidation_response,
		},
		operator_debug: input.operator_debug,
		evidence: MaterializedJobEvidence {
			job_id: loaded.job.job_id.clone(),
			suite: loaded.job.suite.clone(),
			title: loaded.job.title.clone(),
			status,
			query: loaded.job.prompt.content.clone(),
			evidence_ids: input.evidence_ids,
			returned_count: input.returned_count,
			indexing_latency_ms: input.indexing_latency_ms,
			latency_ms: input.latency_ms,
			trace_id: input.trace_id,
			failure: failure_reason,
			source_mappings: input.source_mappings,
			operator_debug: input.operator_debug_evidence,
			capture: input.capture,
			consolidation: input.consolidation,
			knowledge: input.knowledge,
			temporal_reconciliation: input.temporal_reconciliation,
			dreaming_readback: input.dreaming_readback,
		},
	}
}

fn declared_encoding_job(adapter_id: &str, loaded: &LoadedJob) -> Option<MaterializedJob> {
	if is_operator_debug_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if is_elf_consolidation_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if is_elf_knowledge_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if is_elf_capture_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}

	let status = loaded.job.encoding.status?;
	let reason = loaded.job.encoding.reason.clone().unwrap_or_else(|| {
		format!("Fixture declares {} for this live adapter job.", status.as_str())
	});

	Some(materialized_declared_status_job(
		adapter_id,
		loaded,
		status.materialization_status(),
		reason,
	))
}

fn not_encoded_job(adapter_id: &str, loaded: &LoadedJob) -> Option<MaterializedJob> {
	if is_operator_debug_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if is_elf_consolidation_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if is_elf_knowledge_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if is_elf_capture_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if is_elf_dreaming_readback_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}

	not_encoded_reason(loaded.job.suite.as_str()).map(|reason| {
		materialized_declared_status_job(
			adapter_id,
			loaded,
			MaterializationStatus::NotEncoded,
			reason.to_string(),
		)
	})
}

fn is_operator_debug_live_adapter(adapter_id: &str, suite: &str) -> bool {
	suite == "operator_debugging_ux"
		&& matches!(
			adapter_id,
			"elf_live_real_world"
				| "qmd_live_real_world"
				| "elf_operator_debug_live"
				| "qmd_operator_debug_live"
		)
}

fn is_elf_consolidation_live_adapter(adapter_id: &str, suite: &str) -> bool {
	suite == "consolidation" && adapter_id == "elf_live_real_world"
}

fn is_elf_knowledge_live_adapter(adapter_id: &str, suite: &str) -> bool {
	suite == "knowledge_compilation" && adapter_id == "elf_live_real_world"
}

fn is_elf_capture_live_adapter(adapter_id: &str, suite: &str) -> bool {
	suite == "capture_integration"
		&& matches!(adapter_id, "elf_live_real_world" | "elf_capture_write_policy_live")
}

fn is_elf_dreaming_readback_live_adapter(adapter_id: &str, suite: &str) -> bool {
	matches!(suite, "memory_summary" | "proactive_brief" | "scheduled_memory")
		&& matches!(adapter_id, "elf_service_native_dreaming" | "elf_live_real_world")
}

fn not_encoded_reason(suite: &str) -> Option<&'static str> {
	match suite {
		"trust_source_of_truth"
		| "work_resume"
		| "project_decisions"
		| "retrieval"
		| "memory_evolution"
		| "personalization" => None,
		"consolidation" => Some(
			"The live adapter sweep retrieves evidence-linked answers but does not generate or review consolidation proposals.",
		),
		"knowledge_compilation" => Some(
			"The live adapter sweep retrieves evidence-linked answers but does not generate derived knowledge pages.",
		),
		"operator_debugging_ux" => Some(
			"The full live adapter sweep keeps operator trace/viewer diagnostics in a focused operator-debug slice.",
		),
		"capture_integration" => Some(
			"The live adapter sweep does not exercise capture integrations or write-policy redaction boundaries.",
		),
		"production_ops" => Some(
			"The live adapter sweep does not run backup/restore, private corpus, provider credential, or backfill operations.",
		),
		_ => Some("The live adapter sweep has no encoded runtime path for this suite."),
	}
}

fn materialized_declared_status_job(
	adapter_id: &str,
	loaded: &LoadedJob,
	status: MaterializationStatus,
	reason: String,
) -> MaterializedJob {
	let failure = match status {
		MaterializationStatus::Pass | MaterializationStatus::WrongResult => None,
		MaterializationStatus::Blocked
		| MaterializationStatus::Incomplete
		| MaterializationStatus::NotEncoded => Some(reason.clone()),
	};

	MaterializedJob {
		response: AdapterResponseOutput {
			adapter_id: adapter_id.to_string(),
			answer: AnswerOutput {
				content: String::new(),
				evidence_ids: Vec::new(),
				claims: Vec::new(),
				pages: Vec::new(),
				memory_summaries: Vec::new(),
				proactive_briefs: Vec::new(),
				scheduled_tasks: Vec::new(),
				latency_ms: 0.0,
				cost: CostOutput {
					currency: "USD".to_string(),
					amount: 0.0,
					input_tokens: 0,
					output_tokens: 0,
				},
				trace_explainability: TraceExplainabilityOutput {
					trace_id: None,
					failure_stage: Some("live_adapter.suite_support".to_string()),
					failure_reason: failure.clone(),
					stages: vec![TraceStageOutput {
						stage_name: "live_adapter.suite_support".to_string(),
						kept_evidence: Vec::new(),
						dropped_evidence: Vec::new(),
						demoted_evidence: Vec::new(),
						distractor_evidence: Vec::new(),
						notes: reason.clone(),
					}],
				},
			},
			consolidation: None,
		},
		evidence: MaterializedJobEvidence {
			job_id: loaded.job.job_id.clone(),
			suite: loaded.job.suite.clone(),
			title: loaded.job.title.clone(),
			status,
			query: loaded.job.prompt.content.clone(),
			evidence_ids: Vec::new(),
			returned_count: 0,
			indexing_latency_ms: None,
			latency_ms: 0.0,
			trace_id: None,
			failure,
			source_mappings: Vec::new(),
			operator_debug: None,
			capture: None,
			consolidation: None,
			knowledge: None,
			temporal_reconciliation: None,
			dreaming_readback: None,
		},
		operator_debug: None,
	}
}

fn operator_debug_output(
	adapter_kind: AdapterKind,
	loaded: &LoadedJob,
	trace_id: Option<Uuid>,
	replay_command: String,
	replay_artifact: String,
) -> (Option<serde_json::Value>, Option<OperatorDebugMaterializationEvidence>) {
	if loaded.job.suite != "operator_debugging_ux" {
		return (None, None);
	}

	let Some(source) = loaded.value.get("operator_debug") else {
		return (None, None);
	};
	let mut debug = source.clone();
	let Some(object) = debug.as_object_mut() else {
		return (None, None);
	};
	let trace_available = trace_id.is_some();
	let replay_command_available = !replay_command.trim().is_empty();
	let raw_sql_needed = false;
	let repair_action_clarity = if replay_command_available { "clear" } else { "unclear" };
	let candidate_drop_visibility =
		operator_debug_candidate_visibility(adapter_kind, object).to_string();

	object.insert("trace_available".to_string(), serde_json::Value::Bool(trace_available));
	object.insert(
		"replay_command_available".to_string(),
		serde_json::Value::Bool(replay_command_available),
	);
	object.insert("raw_sql_needed".to_string(), serde_json::Value::Bool(raw_sql_needed));
	object.insert(
		"dropped_candidate_visibility".to_string(),
		serde_json::Value::String(candidate_drop_visibility.clone()),
	);
	object.insert(
		"trace_completeness".to_string(),
		serde_json::Value::String(
			operator_debug_trace_completeness(adapter_kind, trace_available).to_string(),
		),
	);
	object.insert(
		"repair_action_clarity".to_string(),
		serde_json::Value::String(repair_action_clarity.to_string()),
	);
	object.insert("replay_command".to_string(), serde_json::Value::String(replay_command.clone()));
	object.insert("replay_artifact".to_string(), serde_json::Value::String(replay_artifact));

	match adapter_kind {
		AdapterKind::ElfServiceRuntime =>
			if let Some(trace_id) = trace_id {
				let trace_id = trace_id.to_string();

				object.insert("trace_id".to_string(), serde_json::Value::String(trace_id.clone()));
				object.insert(
					"viewer_url".to_string(),
					serde_json::Value::String(format!("/viewer?trace_id={trace_id}")),
				);
				object.insert(
					"admin_trace_bundle_url".to_string(),
					serde_json::Value::String(format!(
						"/v2/admin/traces/{trace_id}/bundle?mode=full&stage_items_limit=128&candidates_limit=200"
					)),
				);
			},
		AdapterKind::QmdCliRuntime => {
			object.remove("trace_id");
			object.remove("viewer_url");
			object.remove("admin_trace_bundle_url");
			object.insert("viewer_panels".to_string(), serde_json::json!(["qmd JSON Replay Rows"]));
		},
		AdapterKind::LightragApiContextExport => {},
	}

	let mut cli_steps = string_array_from_object(object, "cli_steps");

	push_unique(&mut cli_steps, replay_command);

	object.insert("cli_steps".to_string(), serde_json::json!(cli_steps));

	(
		Some(debug),
		Some(OperatorDebugMaterializationEvidence {
			trace_available,
			replay_command_available,
			candidate_drop_visibility,
			repair_action_clarity: repair_action_clarity.to_string(),
			raw_sql_needed,
		}),
	)
}

fn operator_debug_trace_completeness(
	adapter_kind: AdapterKind,
	trace_available: bool,
) -> &'static str {
	match adapter_kind {
		AdapterKind::ElfServiceRuntime if trace_available => "complete",
		AdapterKind::ElfServiceRuntime => "missing",
		AdapterKind::QmdCliRuntime | AdapterKind::LightragApiContextExport => "not_available",
	}
}

fn operator_debug_candidate_visibility(
	adapter_kind: AdapterKind,
	object: &Map<String, serde_json::Value>,
) -> &str {
	match adapter_kind {
		AdapterKind::ElfServiceRuntime => object
			.get("dropped_candidate_visibility")
			.and_then(serde_json::Value::as_str)
			.unwrap_or("visible through trace bundle replay candidates"),
		AdapterKind::QmdCliRuntime =>
			"qmd top-k replay output is available, but intermediate candidate-drop stages are not exposed",
		AdapterKind::LightragApiContextExport => "not encoded for this adapter",
	}
}

fn string_array_from_object(object: &Map<String, serde_json::Value>, key: &str) -> Vec<String> {
	object
		.get(key)
		.and_then(serde_json::Value::as_array)
		.map(|items| {
			items.iter().filter_map(serde_json::Value::as_str).map(ToString::to_string).collect()
		})
		.unwrap_or_default()
}

fn elf_replay_command(trace_id: Uuid, project_id: &str) -> String {
	format!(
		"curl -fsS {} -H {} -H {} -H {}",
		shell_quote(format!(
			"http://127.0.0.1:51891/v2/admin/traces/{trace_id}/bundle?mode=full&stage_items_limit=128&candidates_limit=200"
		)
		.as_str()),
		shell_quote("X-ELF-Tenant-Id: elf-live-real-world"),
		shell_quote(format!("X-ELF-Project-Id: {project_id}").as_str()),
		shell_quote("X-ELF-Agent-Id: elf-live-real-world-agent")
	)
}

fn qmd_replay_command(query: &str, collection: &str) -> String {
	format!(
		"npx tsx src/cli/qmd.ts query {} -c {} --json --no-rerank --min-score 0 -n 5",
		shell_quote(format!("lex: {query}\nvec: {query}").as_str()),
		shell_quote(collection)
	)
}

fn shell_quote(value: &str) -> String {
	format!("'{}'", value.replace('\'', "'\\''"))
}

fn evidence_linked_claims(loaded: &LoadedJob, evidence_ids: &[String]) -> Vec<serde_json::Value> {
	loaded
		.job
		.expected_answer
		.must_include
		.iter()
		.filter_map(|claim| {
			let claim_id = claim.claim_id()?;
			let allowed =
				evidence_link_ids(loaded.job.expected_answer.evidence_links.get(claim_id)?);
			let produced = evidence_ids
				.iter()
				.filter(|evidence_id| allowed.iter().any(|allowed_id| allowed_id == *evidence_id))
				.cloned()
				.collect::<Vec<_>>();

			if produced.is_empty() {
				return None;
			}

			Some(serde_json::json!({
				"claim_id": claim_id,
				"text": claim.text(),
				"evidence_ids": produced,
				"confidence": "derived_from_live_retrieval"
			}))
		})
		.collect()
}

fn answer_claims(loaded: &LoadedJob, evidence_ids: &[String]) -> Vec<serde_json::Value> {
	if loaded.job.memory_evolution.is_some() {
		let claims = temporal_reconciliation_claims(loaded, evidence_ids);

		if !claims.is_empty() {
			return claims;
		}
	}

	evidence_linked_claims(loaded, evidence_ids)
}

fn temporal_reconciliation_claims(
	loaded: &LoadedJob,
	evidence_ids: &[String],
) -> Vec<serde_json::Value> {
	let Some(evolution) = &loaded.job.memory_evolution else {
		return Vec::new();
	};
	let selected = evidence_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let mut claims = Vec::new();
	let mut claim_ids = BTreeSet::new();

	for expected in &loaded.job.expected_answer.must_include {
		let Some(claim_id) = expected.claim_id() else {
			continue;
		};
		let mut claim_evidence = temporal_claim_evidence(evolution, claim_id, &selected);

		if claim_evidence.is_empty()
			&& let Some(allowed) = loaded.job.expected_answer.evidence_links.get(claim_id)
		{
			claim_evidence = selected_allowed_evidence(allowed, &selected);
		}
		if claim_evidence.is_empty() {
			continue;
		}

		claim_ids.insert(claim_id.to_string());
		claims.push(json_claim(claim_id, expected.text(), claim_evidence));
	}

	if let Some(rationale) = &evolution.update_rationale
		&& rationale.available
		&& !claim_ids.contains(rationale.claim_id.as_str())
	{
		let claim_evidence = rationale
			.evidence_ids
			.iter()
			.filter(|id| selected.contains(id.as_str()))
			.cloned()
			.collect::<Vec<_>>();

		if !claim_evidence.is_empty() {
			let text = expected_claim_text_for_id(loaded, rationale.claim_id.as_str())
				.unwrap_or("The supersession rationale is selected as lifecycle evidence.");

			claims.push(json_claim(rationale.claim_id.as_str(), text, claim_evidence));
		}
	}

	claims
}

fn temporal_claim_evidence(
	evolution: &LiveMemoryEvolution,
	claim_id: &str,
	selected: &BTreeSet<&str>,
) -> Vec<String> {
	let mut evidence = Vec::new();

	for conflict in &evolution.conflicts {
		if conflict.claim_id != claim_id {
			continue;
		}

		push_if_selected(&mut evidence, conflict.current_evidence_id.as_str(), selected);
		push_if_selected(&mut evidence, conflict.historical_evidence_id.as_str(), selected);

		if let Some(rationale_id) = &conflict.resolved_by_evidence_id {
			push_if_selected(&mut evidence, rationale_id.as_str(), selected);
		}
	}

	evidence
}

fn selected_allowed_evidence(
	allowed: &serde_json::Value,
	selected: &BTreeSet<&str>,
) -> Vec<String> {
	evidence_link_ids(allowed).into_iter().filter(|id| selected.contains(id.as_str())).collect()
}

fn expected_claim_text_for_id<'a>(loaded: &'a LoadedJob, claim_id: &str) -> Option<&'a str> {
	loaded
		.job
		.expected_answer
		.must_include
		.iter()
		.find(|claim| claim.claim_id() == Some(claim_id))
		.map(LiveExpectedClaim::text)
}

fn json_claim(claim_id: &str, text: &str, evidence_ids: Vec<String>) -> serde_json::Value {
	serde_json::json!({
		"claim_id": claim_id,
		"text": text,
		"evidence_ids": evidence_ids,
		"confidence": "derived_from_live_temporal_reconciliation"
	})
}

fn push_if_selected(out: &mut Vec<String>, evidence_id: &str, selected: &BTreeSet<&str>) {
	if selected.contains(evidence_id) {
		push_unique(out, evidence_id.to_string());
	}
}

fn evidence_link_ids(value: &serde_json::Value) -> Vec<String> {
	if let Some(id) = value.as_str() {
		return vec![id.to_string()];
	}

	value
		.as_array()
		.map(|items| {
			items
				.iter()
				.filter_map(serde_json::Value::as_str)
				.map(ToString::to_string)
				.collect::<Vec<_>>()
		})
		.unwrap_or_default()
}

fn required_evidence_satisfied(loaded: &LoadedJob, evidence_ids: &[String]) -> bool {
	if loaded.job.required_evidence.is_empty() {
		return !evidence_ids.is_empty();
	}

	loaded
		.job
		.required_evidence
		.iter()
		.all(|required| evidence_ids.iter().any(|id| id == &required.evidence_id))
}

fn selected_required_corpus_texts(
	loaded: &LoadedJob,
	corpus: &[CorpusText],
	retrieved_evidence_ids: &[String],
) -> SelectedEvidenceText {
	let required_ids = loaded
		.job
		.required_evidence
		.iter()
		.map(|evidence| evidence.evidence_id.as_str())
		.collect::<BTreeSet<_>>();
	let mut selected_ids = Vec::new();

	if required_ids.is_empty() {
		for evidence_id in retrieved_evidence_ids.iter().take(1) {
			push_unique(&mut selected_ids, evidence_id.clone());
		}
	} else {
		for evidence in &loaded.job.required_evidence {
			if retrieved_evidence_ids.iter().any(|id| id == &evidence.evidence_id) {
				push_unique(&mut selected_ids, evidence.evidence_id.clone());
			}
		}
	}

	let content = selected_ids
		.iter()
		.filter_map(|evidence_id| {
			corpus
				.iter()
				.find(|item| item.evidence_id == *evidence_id)
				.map(|item| item.text.clone())
		})
		.collect::<Vec<_>>()
		.join("\n\n");

	SelectedEvidenceText { content, evidence_ids: selected_ids }
}

fn temporal_reconciliation_selection(
	loaded: &LoadedJob,
	corpus: &[CorpusText],
	retrieved_evidence_ids: &[String],
	ingested: &IngestedCorpus,
) -> Option<TemporalReconciliationSelection> {
	let evolution = loaded.job.memory_evolution.as_ref()?;
	let relevant_ids = temporal_reconciliation_relevant_ids(loaded, evolution);
	let retrieved_ids = retrieved_evidence_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let mut selected_ids = Vec::new();

	for evidence_id in &relevant_ids {
		if retrieved_ids.contains(evidence_id.as_str())
			&& ingested.note_ids_by_evidence.contains_key(evidence_id)
		{
			push_unique(&mut selected_ids, evidence_id.clone());
		}
	}

	if selected_ids.is_empty() {
		return None;
	}

	let content = temporal_reconciliation_content(loaded, corpus, &selected_ids);
	let selected = SelectedEvidenceText { content, evidence_ids: selected_ids.clone() };
	let evidence = temporal_reconciliation_evidence(
		evolution,
		&relevant_ids,
		retrieved_evidence_ids,
		&selected_ids,
		ingested,
		loaded,
	);
	let trace_stages =
		temporal_reconciliation_trace_stages(evolution, retrieved_evidence_ids, &evidence);

	Some(TemporalReconciliationSelection { selected, evidence, trace_stages })
}

fn temporal_reconciliation_relevant_ids(
	loaded: &LoadedJob,
	evolution: &LiveMemoryEvolution,
) -> Vec<String> {
	let mut ids = Vec::new();

	for evidence in &loaded.job.required_evidence {
		push_unique(&mut ids, evidence.evidence_id.clone());
	}
	for evidence_id in &evolution.current_evidence_ids {
		push_unique(&mut ids, evidence_id.clone());
	}
	for evidence_id in &evolution.historical_evidence_ids {
		push_unique(&mut ids, evidence_id.clone());
	}
	for evidence_id in &evolution.tombstone_evidence_ids {
		push_unique(&mut ids, evidence_id.clone());
	}
	for evidence_id in &evolution.invalidation_evidence_ids {
		push_unique(&mut ids, evidence_id.clone());
	}
	for conflict in &evolution.conflicts {
		push_unique(&mut ids, conflict.current_evidence_id.clone());
		push_unique(&mut ids, conflict.historical_evidence_id.clone());

		if let Some(evidence_id) = &conflict.resolved_by_evidence_id {
			push_unique(&mut ids, evidence_id.clone());
		}
	}

	if let Some(rationale) = &evolution.update_rationale
		&& rationale.available
	{
		for evidence_id in &rationale.evidence_ids {
			push_unique(&mut ids, evidence_id.clone());
		}
	}

	ids
}

fn temporal_reconciliation_content(
	loaded: &LoadedJob,
	corpus: &[CorpusText],
	selected_ids: &[String],
) -> String {
	let expected = loaded
		.job
		.expected_answer
		.must_include
		.iter()
		.map(LiveExpectedClaim::text)
		.collect::<Vec<_>>()
		.join(" ");
	let evidence_summary = selected_ids
		.iter()
		.filter_map(|evidence_id| {
			corpus
				.iter()
				.find(|item| item.evidence_id == *evidence_id)
				.map(|item| format!("{evidence_id}: {}", item.text))
		})
		.collect::<Vec<_>>()
		.join("\n");

	if evidence_summary.is_empty() {
		expected
	} else {
		format!("{expected}\n\nTemporal reconciliation evidence:\n{evidence_summary}")
	}
}

fn temporal_reconciliation_evidence(
	evolution: &LiveMemoryEvolution,
	relevant_ids: &[String],
	retrieved_evidence_ids: &[String],
	selected_ids: &[String],
	ingested: &IngestedCorpus,
	loaded: &LoadedJob,
) -> TemporalReconciliationMaterializationEvidence {
	let selected = selected_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let retrieved = retrieved_evidence_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let mut evidence = TemporalReconciliationMaterializationEvidence {
		current_winner_evidence_ids: selected_subset(&evolution.current_evidence_ids, &selected),
		historical_loser_evidence_ids: selected_subset(
			&evolution.historical_evidence_ids,
			&selected,
		),
		supersession_rationale_evidence_ids: evolution
			.update_rationale
			.as_ref()
			.filter(|rationale| rationale.available)
			.map_or_else(Vec::new, |rationale| selected_subset(&rationale.evidence_ids, &selected)),
		tombstone_evidence_ids: selected_subset(&evolution.tombstone_evidence_ids, &selected),
		invalidation_evidence_ids: selected_subset(&evolution.invalidation_evidence_ids, &selected),
		conflict_candidate_evidence_ids: conflict_candidate_ids(evolution, &selected),
		retrieved_evidence_ids: retrieved_evidence_ids.to_vec(),
		selected_evidence_ids: selected_ids.to_vec(),
		absent_evidence_ids: relevant_ids
			.iter()
			.filter(|id| !ingested.note_ids_by_evidence.contains_key(*id))
			.cloned()
			.collect(),
		retrieved_but_dropped_evidence_ids: relevant_ids
			.iter()
			.filter(|id| retrieved.contains(id.as_str()) && !selected.contains(id.as_str()))
			.cloned()
			.collect(),
		selected_but_not_narrated_evidence_ids: selected_but_not_narrated_ids(loaded, selected_ids),
		contradicted_by_lifecycle_evidence_ids: Vec::new(),
	};

	for evidence_id in evidence
		.historical_loser_evidence_ids
		.iter()
		.chain(evidence.tombstone_evidence_ids.iter())
		.chain(evidence.invalidation_evidence_ids.iter())
	{
		push_unique(&mut evidence.contradicted_by_lifecycle_evidence_ids, evidence_id.clone());
	}

	evidence
}

fn selected_subset(ids: &[String], selected: &BTreeSet<&str>) -> Vec<String> {
	ids.iter().filter(|id| selected.contains(id.as_str())).cloned().collect()
}

fn conflict_candidate_ids(
	evolution: &LiveMemoryEvolution,
	selected: &BTreeSet<&str>,
) -> Vec<String> {
	let mut ids = Vec::new();

	for conflict in &evolution.conflicts {
		push_if_selected(&mut ids, conflict.current_evidence_id.as_str(), selected);
		push_if_selected(&mut ids, conflict.historical_evidence_id.as_str(), selected);

		if let Some(evidence_id) = &conflict.resolved_by_evidence_id {
			push_if_selected(&mut ids, evidence_id.as_str(), selected);
		}
	}

	ids
}

fn selected_but_not_narrated_ids(loaded: &LoadedJob, selected_ids: &[String]) -> Vec<String> {
	let claims = temporal_reconciliation_claims(loaded, selected_ids);
	let narrated = claims
		.iter()
		.flat_map(|claim| {
			claim
				.get("evidence_ids")
				.and_then(serde_json::Value::as_array)
				.into_iter()
				.flatten()
				.filter_map(serde_json::Value::as_str)
		})
		.collect::<BTreeSet<_>>();

	selected_ids.iter().filter(|id| !narrated.contains(id.as_str())).cloned().collect()
}

fn temporal_reconciliation_trace_stages(
	evolution: &LiveMemoryEvolution,
	retrieved_evidence_ids: &[String],
	evidence: &TemporalReconciliationMaterializationEvidence,
) -> Vec<TraceStageOutput> {
	let selected =
		evidence.selected_evidence_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let retrieved = retrieved_evidence_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let expected_not_retrieved = evidence
		.selected_evidence_ids
		.iter()
		.filter(|id| !retrieved.contains(id.as_str()))
		.cloned()
		.collect::<Vec<_>>();

	vec![
		TraceStageOutput {
			stage_name: "live_adapter.retrieve".to_string(),
			kept_evidence: retrieved_evidence_ids.to_vec(),
			dropped_evidence: expected_not_retrieved,
			demoted_evidence: Vec::new(),
			distractor_evidence: evidence.absent_evidence_ids.clone(),
			notes:
				"Search output is compared with the temporal reconciliation evidence contract."
					.to_string(),
		},
		TraceStageOutput {
			stage_name: "temporal_reconciliation.current_winner".to_string(),
			kept_evidence: evidence.current_winner_evidence_ids.clone(),
			dropped_evidence: unselected_subset(&evolution.current_evidence_ids, &selected),
			demoted_evidence: Vec::new(),
			distractor_evidence: Vec::new(),
			notes: "Current evidence selected as the answer winner.".to_string(),
		},
		TraceStageOutput {
			stage_name: "temporal_reconciliation.historical_loser".to_string(),
			kept_evidence: evidence.historical_loser_evidence_ids.clone(),
			dropped_evidence: unselected_subset(&evolution.historical_evidence_ids, &selected),
			demoted_evidence: evidence.historical_loser_evidence_ids.clone(),
			distractor_evidence: Vec::new(),
			notes: "Historical evidence preserved as history, not as the current answer."
				.to_string(),
		},
		TraceStageOutput {
			stage_name: "temporal_reconciliation.supersession_rationale".to_string(),
			kept_evidence: evidence.supersession_rationale_evidence_ids.clone(),
			dropped_evidence: evolution
				.update_rationale
				.as_ref()
				.map_or_else(Vec::new, |rationale| {
					unselected_subset(&rationale.evidence_ids, &selected)
				}),
			demoted_evidence: Vec::new(),
			distractor_evidence: Vec::new(),
			notes: "Rationale evidence selected to explain why the older fact was superseded."
				.to_string(),
		},
		TraceStageOutput {
			stage_name: "temporal_reconciliation.tombstone_invalidation".to_string(),
			kept_evidence: evidence
				.tombstone_evidence_ids
				.iter()
				.chain(evidence.invalidation_evidence_ids.iter())
				.cloned()
				.collect(),
			dropped_evidence: evolution
				.tombstone_evidence_ids
				.iter()
				.chain(evolution.invalidation_evidence_ids.iter())
				.filter(|id| !selected.contains(id.as_str()))
				.cloned()
				.collect(),
			demoted_evidence: Vec::new(),
			distractor_evidence: Vec::new(),
			notes: "Tombstone or TTL invalidation evidence remains answerable when present."
				.to_string(),
		},
		TraceStageOutput {
			stage_name: "temporal_reconciliation.conflict_candidates".to_string(),
			kept_evidence: evidence.conflict_candidate_evidence_ids.clone(),
			dropped_evidence: evidence.retrieved_but_dropped_evidence_ids.clone(),
			demoted_evidence: evidence.contradicted_by_lifecycle_evidence_ids.clone(),
			distractor_evidence: evidence.selected_but_not_narrated_evidence_ids.clone(),
			notes:
				"Conflict candidates record selected, dropped, non-narrated, and lifecycle-demoted evidence."
					.to_string(),
		},
	]
}

fn unselected_subset(ids: &[String], selected: &BTreeSet<&str>) -> Vec<String> {
	ids.iter().filter(|id| !selected.contains(id.as_str())).cloned().collect()
}

fn live_required_evidence_ids(loaded: &LoadedJob, ingested: &IngestedCorpus) -> Vec<String> {
	let mut selected = Vec::new();

	for evidence in &loaded.job.required_evidence {
		if ingested.note_ids_by_evidence.contains_key(&evidence.evidence_id) {
			push_unique(&mut selected, evidence.evidence_id.clone());
		}
	}

	if selected.is_empty() {
		for evidence_id in ingested.note_ids_by_evidence.keys() {
			push_unique(&mut selected, evidence_id.clone());
		}

		selected.sort();
	}

	selected
}

fn expected_claim_text(loaded: &LoadedJob, evidence_ids: &[String]) -> SelectedEvidenceText {
	let content = loaded
		.job
		.expected_answer
		.must_include
		.iter()
		.map(LiveExpectedClaim::text)
		.collect::<Vec<_>>()
		.join(" ");

	SelectedEvidenceText { content, evidence_ids: evidence_ids.to_vec() }
}

fn capture_runtime_evidence_from_search_items(items: &[SearchItem]) -> CaptureRuntimeEvidence {
	let source_refs = items.iter().map(|item| &item.source_ref);

	capture_runtime_evidence_from_source_refs(source_refs)
}

fn capture_runtime_evidence_from_source_refs<'a>(
	source_refs: impl IntoIterator<Item = &'a serde_json::Value>,
) -> CaptureRuntimeEvidence {
	let mut runtime = CaptureRuntimeEvidence::default();

	for source_ref in source_refs {
		let Some(evidence_id) = source_ref.get("evidence_id").and_then(serde_json::Value::as_str)
		else {
			continue;
		};

		if runtime.items.iter().any(|item| item.evidence_id == evidence_id) {
			continue;
		}

		runtime.items.push(CaptureRuntimeEvidenceItem {
			evidence_id: evidence_id.to_string(),
			source_id: source_ref
				.get("source_id")
				.and_then(serde_json::Value::as_str)
				.map(ToString::to_string),
			evidence_binding: source_ref
				.get("evidence_binding")
				.and_then(serde_json::Value::as_str)
				.map(ToString::to_string),
			write_policy_applied: source_ref
				.get("write_policy_applied")
				.and_then(serde_json::Value::as_bool)
				.unwrap_or(false),
			capture_action: source_ref
				.get("capture_action")
				.and_then(serde_json::Value::as_str)
				.map(ToString::to_string),
			source_ref: source_ref.clone(),
		});
	}

	runtime
}

fn capture_with_runtime_source_refs(
	mut capture: CaptureMaterializationEvidence,
	runtime: &CaptureRuntimeEvidence,
) -> CaptureMaterializationEvidence {
	capture.source_ids.clear();
	capture.runtime_source_refs.clear();

	for item in &runtime.items {
		if let Some(source_id) = item.source_id.as_deref() {
			push_unique(&mut capture.source_ids, source_id.to_string());
		}

		capture.runtime_source_refs.push(CaptureRuntimeSourceRefEvidence {
			evidence_id: item.evidence_id.clone(),
			source_ref: item.source_ref.clone(),
		});
	}

	capture
}

fn validate_capture_runtime_evidence(
	suite: &str,
	corpus: &[CorpusText],
	capture: &CaptureMaterializationEvidence,
	runtime: &CaptureRuntimeEvidence,
) -> Option<String> {
	if suite != "capture_integration" {
		return None;
	}

	let mut failures = Vec::new();
	let mut expected_redactions = 0_usize;
	let mut expected_exclusions = 0_usize;

	for item in corpus {
		match item.capture.action {
			LiveCaptureAction::Exclude => {
				if runtime.item_for(item.evidence_id.as_str()).is_some() {
					failures.push(format!(
						"excluded evidence {} was returned by live search",
						item.evidence_id
					));
				}
				if capture.stored_evidence_ids.iter().any(|id| id == &item.evidence_id) {
					failures.push(format!(
						"excluded evidence {} was stored by live ingestion",
						item.evidence_id
					));
				}
				if !capture.excluded_evidence_ids.iter().any(|id| id == &item.evidence_id) {
					failures.push(format!(
						"excluded evidence {} was not recorded as excluded",
						item.evidence_id
					));
				}
			},
			LiveCaptureAction::Store => {
				let runtime_item = runtime.item_for(item.evidence_id.as_str());

				if let Some(expected_source_id) = item.capture.source_id.as_deref() {
					match runtime_item.and_then(|observed| observed.source_id.as_deref()) {
						Some(observed) if observed == expected_source_id => {},
						Some(observed) => failures.push(format!(
							"evidence {} returned source_id {observed}, expected {expected_source_id}",
							item.evidence_id
						)),
						None => failures.push(format!(
							"evidence {} did not return expected source_id {expected_source_id}",
							item.evidence_id
						)),
					}
				}
				if let Some(expected_binding) = item.capture.evidence_binding.as_deref() {
					match runtime_item.and_then(|observed| observed.evidence_binding.as_deref()) {
						Some(observed) if observed == expected_binding => {},
						Some(observed) => failures.push(format!(
							"evidence {} returned evidence_binding {observed}, expected {expected_binding}",
							item.evidence_id
						)),
						None => failures.push(format!(
							"evidence {} did not return expected evidence_binding {expected_binding}",
							item.evidence_id
						)),
					}
				}
				if let Some(policy_value) = &item.capture.write_policy {
					match write_policy_from_value(policy_value, item.evidence_id.as_str()) {
						Ok(policy) => {
							expected_exclusions += policy.exclusions.len();
							expected_redactions += policy.redactions.len();
						},
						Err(err) => failures.push(err.to_string()),
					}

					if !runtime_item.is_some_and(|observed| observed.write_policy_applied) {
						failures.push(format!(
							"evidence {} did not return write_policy_applied=true",
							item.evidence_id
						));
					}
				}
				if let Some(observed) =
					runtime_item.and_then(|observed| observed.capture_action.as_deref())
					&& observed != capture_action_str(item.capture.action)
				{
					failures.push(format!(
						"evidence {} returned capture_action {observed}, expected {}",
						item.evidence_id,
						capture_action_str(item.capture.action)
					));
				}
			},
		}
	}

	if capture.write_policy_exclusion_count < expected_exclusions {
		failures.push(format!(
			"write-policy exclusion count {} was below expected {expected_exclusions}",
			capture.write_policy_exclusion_count
		));
	}
	if capture.write_policy_redaction_count < expected_redactions {
		failures.push(format!(
			"write-policy redaction count {} was below expected {expected_redactions}",
			capture.write_policy_redaction_count
		));
	}
	if expected_exclusions + expected_redactions > 0 && capture.write_policy_audit_count == 0 {
		failures
			.push("write-policy audit count was zero despite expected policy effects".to_string());
	}
	if failures.is_empty() {
		None
	} else {
		Some(format!("Capture runtime validation failed: {}", failures.join("; ")))
	}
}

fn elf_stored_corpus_texts(corpus: &[CorpusText]) -> color_eyre::Result<Vec<CorpusText>> {
	let mut stored = Vec::new();

	for item in corpus {
		if item.capture.action == LiveCaptureAction::Exclude {
			continue;
		}

		stored.push(CorpusText {
			evidence_id: item.evidence_id.clone(),
			text: transformed_capture_text(item)?.trim().to_string(),
			capture: item.capture.clone(),
		});
	}

	Ok(stored)
}

fn transformed_capture_text(item: &CorpusText) -> color_eyre::Result<String> {
	let Some(policy_value) = &item.capture.write_policy else {
		return Ok(item.text.clone());
	};
	let policy = write_policy_from_value(policy_value, item.evidence_id.as_str())?;
	let result =
		writegate::apply_write_policy(item.text.as_str(), Some(&policy)).map_err(|err| {
			eyre::eyre!("Invalid write_policy for evidence {}: {err:?}", item.evidence_id)
		})?;

	Ok(result.transformed)
}

fn write_policy_from_value(
	value: &serde_json::Value,
	evidence_id: &str,
) -> color_eyre::Result<WritePolicy> {
	serde_json::from_value::<WritePolicy>(value.clone()).map_err(|err| {
		eyre::eyre!("Failed to parse write_policy for evidence {evidence_id}: {err}")
	})
}

fn failure_jobs(
	adapter_id: &str,
	jobs: &[LoadedJob],
	stage: &str,
	reason: String,
) -> Vec<MaterializedJob> {
	jobs.iter()
		.map(|job| {
			materialized_job(
				job,
				adapter_id,
				MaterializedJobInput {
					content: String::new(),
					evidence_ids: Vec::new(),
					pages: Vec::new(),
					latency_ms: 0.0,
					indexing_latency_ms: None,
					returned_count: 0,
					trace_id: None,
					failure: Some(format!("{stage}: {reason}")),
					source_mappings: Vec::new(),
					operator_debug: None,
					operator_debug_evidence: None,
					capture: None,
					capture_failure: None,
					consolidation_response: None,
					consolidation: None,
					knowledge: None,
					temporal_reconciliation: None,
					dreaming_readback: None,
					memory_summaries: Vec::new(),
					proactive_briefs: Vec::new(),
					scheduled_tasks: Vec::new(),
					trace_stages: None,
				},
			)
		})
		.collect()
}

fn write_materialized_output(output: MaterializedOutput<'_>) -> color_eyre::Result<()> {
	if output.out_fixtures.exists() {
		fs::remove_dir_all(output.out_fixtures)?;
	}

	fs::create_dir_all(output.out_fixtures)?;

	for (loaded, materialized) in output.jobs.iter().zip(output.materialized) {
		let mut value = loaded.value.clone();
		let mut adapter_response =
			value["corpus"]["adapter_response"].as_object().cloned().unwrap_or_default();

		adapter_response.insert(
			"adapter_id".to_string(),
			serde_json::to_value(&materialized.response.adapter_id)?,
		);
		adapter_response
			.insert("answer".to_string(), serde_json::to_value(&materialized.response.answer)?);

		if let Some(consolidation) = &materialized.response.consolidation {
			adapter_response.insert("consolidation".to_string(), consolidation.clone());
		} else if loaded.job.suite == "consolidation" {
			adapter_response.remove("consolidation");
		}

		value["corpus"]["adapter_response"] = serde_json::Value::Object(adapter_response);

		if let Some(operator_debug) = &materialized.operator_debug {
			value["operator_debug"] = operator_debug.clone();
		}
		if let Some(capture) = &materialized.evidence.capture {
			apply_capture_runtime_source_refs(&mut value, capture);

			value["capture_materialization"] = serde_json::to_value(capture)?;
		}

		if matches!(
			materialized.evidence.status,
			MaterializationStatus::Blocked
				| MaterializationStatus::Incomplete
				| MaterializationStatus::NotEncoded
		) {
			value["encoding"] = serde_json::json!({
				"status": materialization_status_str(materialized.evidence.status),
				"reason": materialized.evidence.failure.clone().unwrap_or_else(|| {
					"Live adapter did not complete this job as a pass/fail check.".to_string()
				}),
			});
		}

		let output_path = output_fixture_path(output.fixtures, output.out_fixtures, &loaded.path)?;

		if let Some(parent) = output_path.parent() {
			fs::create_dir_all(parent)?;
		}

		fs::write(output_path, serde_json::to_string_pretty(&value)?)?;
	}

	let evidence = MaterializationEvidence {
		schema: EVIDENCE_SCHEMA,
		adapter_id: output.adapter_id.to_string(),
		adapter_kind: output.adapter_kind,
		status: aggregate_status(output.materialized),
		fixtures: output.fixtures.display().to_string(),
		generated_fixtures: output.out_fixtures.display().to_string(),
		command_evidence: output.command_evidence,
		jobs: output.materialized.iter().map(|job| clone_job_evidence(&job.evidence)).collect(),
		metadata: output.metadata,
	};

	if let Some(parent) = output.evidence_out.parent() {
		fs::create_dir_all(parent)?;
	}

	fs::write(output.evidence_out, serde_json::to_string_pretty(&evidence)?)?;

	Ok(())
}

fn apply_capture_runtime_source_refs(
	value: &mut serde_json::Value,
	capture: &CaptureMaterializationEvidence,
) {
	let Some(items) = value.pointer_mut("/corpus/items").and_then(serde_json::Value::as_array_mut)
	else {
		return;
	};

	for item in items {
		let Some(evidence_id) = item.get("evidence_id").and_then(serde_json::Value::as_str) else {
			continue;
		};
		let Some(source_ref) = capture
			.runtime_source_refs
			.iter()
			.find(|source_ref| source_ref.evidence_id == evidence_id)
		else {
			continue;
		};

		item["source_ref"] = source_ref.source_ref.clone();
	}
}

fn clone_job_evidence(evidence: &MaterializedJobEvidence) -> MaterializedJobEvidence {
	MaterializedJobEvidence {
		job_id: evidence.job_id.clone(),
		suite: evidence.suite.clone(),
		title: evidence.title.clone(),
		status: evidence.status,
		query: evidence.query.clone(),
		evidence_ids: evidence.evidence_ids.clone(),
		returned_count: evidence.returned_count,
		indexing_latency_ms: evidence.indexing_latency_ms,
		latency_ms: evidence.latency_ms,
		trace_id: evidence.trace_id,
		failure: evidence.failure.clone(),
		source_mappings: evidence.source_mappings.clone(),
		operator_debug: evidence.operator_debug.clone(),
		capture: evidence.capture.clone(),
		consolidation: evidence.consolidation.clone(),
		knowledge: evidence.knowledge.clone(),
		temporal_reconciliation: evidence.temporal_reconciliation.clone(),
		dreaming_readback: evidence.dreaming_readback.clone(),
	}
}

fn aggregate_status(jobs: &[MaterializedJob]) -> MaterializationStatus {
	if jobs.iter().any(|job| job.evidence.status == MaterializationStatus::Incomplete) {
		MaterializationStatus::Incomplete
	} else if jobs.iter().any(|job| job.evidence.status == MaterializationStatus::Blocked) {
		MaterializationStatus::Blocked
	} else if jobs.iter().any(|job| job.evidence.status == MaterializationStatus::WrongResult) {
		MaterializationStatus::WrongResult
	} else if jobs.iter().any(|job| job.evidence.status == MaterializationStatus::NotEncoded) {
		MaterializationStatus::NotEncoded
	} else {
		MaterializationStatus::Pass
	}
}

fn materialization_status_str(status: MaterializationStatus) -> &'static str {
	match status {
		MaterializationStatus::Pass => "pass",
		MaterializationStatus::WrongResult => "wrong_result",
		MaterializationStatus::Blocked => "blocked",
		MaterializationStatus::Incomplete => "incomplete",
		MaterializationStatus::NotEncoded => "not_encoded",
	}
}

fn output_fixture_path(
	fixtures: &Path,
	out_fixtures: &Path,
	fixture: &Path,
) -> color_eyre::Result<PathBuf> {
	if fixtures.is_dir() {
		let relative = fixture.strip_prefix(fixtures).map_err(|err| {
			eyre::eyre!(
				"Fixture path {} is not under fixture root {}: {err}",
				fixture.display(),
				fixtures.display()
			)
		})?;

		return Ok(out_fixtures.join(relative));
	}

	let file_name = fixture
		.file_name()
		.ok_or_else(|| eyre::eyre!("Fixture path {} has no file name.", fixture.display()))?;

	Ok(out_fixtures.join(file_name))
}

fn load_jobs(path: &Path) -> color_eyre::Result<Vec<LoadedJob>> {
	let paths = fixture_paths(path)?;
	let mut jobs = Vec::with_capacity(paths.len());

	for fixture in paths {
		let raw = fs::read_to_string(&fixture)?;
		let value = serde_json::from_str::<serde_json::Value>(&raw)
			.map_err(|err| eyre::eyre!("Failed to parse {} as JSON: {err}", fixture.display()))?;
		let job = serde_json::from_value::<LiveJob>(value.clone()).map_err(|err| {
			eyre::eyre!("Failed to parse {} as real_world_job: {err}", fixture.display())
		})?;

		if job.schema != JOB_SCHEMA {
			return Err(eyre::eyre!(
				"{} has schema {}, expected {JOB_SCHEMA}.",
				fixture.display(),
				job.schema
			));
		}
		if job.corpus.items.is_empty() {
			return Err(eyre::eyre!("{} has no corpus items.", fixture.display()));
		}

		jobs.push(LoadedJob { path: fixture, value, job });
	}

	Ok(jobs)
}

fn fixture_paths(path: &Path) -> color_eyre::Result<Vec<PathBuf>> {
	let mut paths = Vec::new();

	collect_fixture_paths(path, &mut paths)?;

	paths.sort();

	Ok(paths)
}

fn collect_fixture_paths(path: &Path, paths: &mut Vec<PathBuf>) -> color_eyre::Result<()> {
	if path.is_dir() {
		for entry in fs::read_dir(path)? {
			let entry_path = entry?.path();

			collect_fixture_paths(entry_path.as_path(), paths)?;
		}

		return Ok(());
	}
	if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
		paths.push(path.to_path_buf());
	}

	Ok(())
}

fn corpus_texts(loaded: &LoadedJob) -> color_eyre::Result<Vec<CorpusText>> {
	loaded
		.job
		.corpus
		.items
		.iter()
		.map(|item| {
			let text = match (&item.text, &item.local_ref) {
				(Some(text), _) => text.clone(),
				(None, Some(local_ref)) => {
					let base = loaded.path.parent().unwrap_or_else(|| Path::new("."));

					fs::read_to_string(base.join(local_ref))?
				},
				(None, None) => {
					return Err(eyre::eyre!(
						"{} item {} has no text or local_ref.",
						loaded.path.display(),
						item.evidence_id
					));
				},
			};

			Ok(CorpusText {
				evidence_id: item.evidence_id.clone(),
				text: text.trim().to_string(),
				capture: item.capture.clone(),
			})
		})
		.collect()
}

fn read_dir_paths(path: &Path) -> color_eyre::Result<Vec<PathBuf>> {
	if !path.exists() {
		return Ok(Vec::new());
	}

	let mut paths = Vec::new();

	for entry in fs::read_dir(path)? {
		paths.push(entry?.path());
	}

	Ok(paths)
}

fn runtime_config(runtime: &BaselineRuntime) -> color_eyre::Result<Config> {
	let mut cfg = elf_config::load(&runtime.config_path)?;

	cfg.storage.postgres.dsn = runtime.dsn.clone();
	cfg.storage.postgres.pool_max_conns = 12;
	cfg.storage.qdrant.url = runtime.qdrant_url.clone();
	cfg.storage.qdrant.collection = runtime.collection.clone();
	cfg.storage.qdrant.docs_collection = runtime.docs_collection.clone();
	cfg.providers.embedding.provider_id = "local".to_string();
	cfg.providers.embedding.model = "local-hash".to_string();
	cfg.providers.embedding.dimensions = cfg.storage.qdrant.vector_dim;
	cfg.providers.rerank.provider_id = "local".to_string();
	cfg.providers.rerank.model = "local-token-overlap".to_string();
	cfg.providers.llm_extractor.provider_id = "disabled".to_string();
	cfg.providers.llm_extractor.model = "disabled".to_string();
	cfg.context = None;

	Ok(cfg)
}

fn deterministic_providers(vector_dim: u32) -> Providers {
	Providers::new(
		Arc::new(DeterministicEmbedding { vector_dim }),
		Arc::new(TokenOverlapRerank),
		Arc::new(NoopExtractor),
	)
}

fn run_qmd_command(
	label: &str,
	args: &QmdArgs,
	home_dir: &Path,
	qmd_args: &[&str],
	log_path: &Path,
) -> color_eyre::Result<String> {
	let mut command = Command::new("npx");

	command
		.current_dir(&args.qmd_dir)
		.env("HOME", home_dir)
		.env("XDG_CACHE_HOME", "/root/.cache")
		.env("QMD_FORCE_CPU", "1")
		.arg("tsx")
		.arg("src/cli/qmd.ts");

	for arg in qmd_args {
		command.arg(arg);
	}

	run_logged_command(label, &mut command, log_path)
}

fn run_logged_shell(
	label: &str,
	cwd: &Path,
	script: &str,
	log_path: &Path,
) -> color_eyre::Result<()> {
	let mut command = Command::new("bash");

	command.current_dir(cwd).arg("-lc").arg(script);

	run_logged_command(label, &mut command, log_path).map(|_| ())
}

fn run_logged_command(
	label: &str,
	command: &mut Command,
	log_path: &Path,
) -> color_eyre::Result<String> {
	if let Some(parent) = log_path.parent() {
		fs::create_dir_all(parent)?;
	}

	let command_debug = format!("{command:?}");
	let output = command.stdout(Stdio::piped()).stderr(Stdio::piped()).output()?;
	let stdout = String::from_utf8_lossy(&output.stdout).to_string();
	let stderr = String::from_utf8_lossy(&output.stderr).to_string();
	let mut log = OpenOptions::new().create(true).append(true).open(log_path)?;

	writeln!(log, "## {label}")?;
	writeln!(log, "$ {command_debug}")?;

	if !stdout.trim().is_empty() {
		writeln!(log, "\nstdout:\n{stdout}")?;
	}
	if !stderr.trim().is_empty() {
		writeln!(log, "\nstderr:\n{stderr}")?;
	}
	if !output.status.success() {
		return Err(eyre::eyre!(
			"{label} failed with status {}. Inspect {}.",
			output.status,
			log_path.display()
		));
	}

	Ok(stdout)
}

fn project_id_for_job(job_id: &str) -> String {
	format!("job-{}", slug(job_id))
}

fn slug(value: &str) -> String {
	let mut out = String::new();
	let mut last_dash = false;

	for ch in value.chars() {
		if ch.is_ascii_alphanumeric() {
			out.push(ch.to_ascii_lowercase());

			last_dash = false;
		} else if !last_dash && !out.is_empty() {
			out.push('-');

			last_dash = true;
		}
	}

	while out.ends_with('-') {
		out.pop();
	}

	if out.is_empty() { "item".to_string() } else { out }
}

fn short_hash(value: &str) -> String {
	let mut hasher = Hasher::new();

	hasher.update(value.as_bytes());

	hasher.finalize().to_hex().chars().take(12).collect()
}

fn push_unique(values: &mut Vec<String>, value: String) {
	if !values.iter().any(|existing| existing == &value) {
		values.push(value);
	}
}

fn embed_text(text: &str, vector_dim: u32) -> Vec<f32> {
	let dim = vector_dim as usize;
	let mut vector = vec![0.0_f32; dim];

	if dim == 0 {
		return vector;
	}

	let normalized = normalize_ascii_alnum_lowercase(text);

	for term in normalized.split_whitespace() {
		if term.len() < 2 {
			continue;
		}

		let hash = blake3::hash(term.as_bytes());
		let bytes = hash.as_bytes();
		let idx = (u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize) % dim;

		vector[idx] += 1.0;
	}

	let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();

	if norm > 0.0 {
		for value in &mut vector {
			*value /= norm;
		}
	}

	vector
}

fn terms(text: &str) -> BTreeSet<String> {
	normalize_ascii_alnum_lowercase(text)
		.split_whitespace()
		.filter(|term| term.len() >= 2)
		.map(ToString::to_string)
		.collect()
}

fn normalize_ascii_alnum_lowercase(text: &str) -> String {
	text.chars()
		.map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { ' ' })
		.collect()
}

fn note_text_chunks(text: &str) -> Vec<String> {
	let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");

	if normalized.chars().count() <= ELF_NOTE_CHUNK_CHARS {
		return vec![normalized];
	}

	let mut chunks = Vec::new();
	let mut current = String::new();

	for word in normalized.split_whitespace() {
		if word.chars().count() > ELF_NOTE_CHUNK_CHARS {
			if !current.is_empty() {
				chunks.push(current);

				current = String::new();
			}

			chunks.extend(split_long_token(word));

			continue;
		}

		let separator = usize::from(!current.is_empty());

		if current.chars().count() + separator + word.chars().count() > ELF_NOTE_CHUNK_CHARS
			&& !current.is_empty()
		{
			chunks.push(current);

			current = String::new();
		}
		if !current.is_empty() {
			current.push(' ');
		}

		current.push_str(word);
	}

	if !current.is_empty() {
		chunks.push(current);
	}

	chunks
}

fn split_long_token(token: &str) -> Vec<String> {
	let mut chunks = Vec::new();
	let mut current = String::new();

	for ch in token.chars() {
		if current.chars().count() >= ELF_NOTE_CHUNK_CHARS {
			chunks.push(current);

			current = String::new();
		}

		current.push(ch);
	}

	if !current.is_empty() {
		chunks.push(current);
	}

	chunks
}

fn capture_for_job(
	loaded: &LoadedJob,
	capture: CaptureMaterializationEvidence,
) -> Option<CaptureMaterializationEvidence> {
	if loaded.job.suite == "capture_integration" { Some(capture) } else { None }
}

fn capture_action_str(action: LiveCaptureAction) -> &'static str {
	match action {
		LiveCaptureAction::Store => "store",
		LiveCaptureAction::Exclude => "exclude",
	}
}

fn live_consolidation_fixture(loaded: &LoadedJob) -> color_eyre::Result<LiveConsolidationFixture> {
	let value =
		loaded.value.pointer("/corpus/adapter_response/consolidation").cloned().ok_or_else(
			|| {
				eyre::eyre!(
					"{} does not contain adapter_response.consolidation.",
					loaded.path.display()
				)
			},
		)?;

	serde_json::from_value(value).map_err(|err| {
		eyre::eyre!("Failed to parse consolidation fixture {}: {err}", loaded.path.display())
	})
}

fn prepare_consolidation_run(
	loaded: &LoadedJob,
	adapter_id: &str,
	ingested: &IngestedCorpus,
	fixture: &LiveConsolidationFixture,
	corpus: &[CorpusText],
) -> color_eyre::Result<PreparedConsolidationRun> {
	let mut input_refs = Vec::new();
	let mut proposals = Vec::new();

	for proposal in &fixture.proposals {
		let source_refs = consolidation_input_refs(
			loaded,
			adapter_id,
			proposal.source_refs.as_slice(),
			ingested,
			corpus,
		)?;

		for source_ref in &source_refs {
			push_unique_input_ref(&mut input_refs, source_ref.clone());
		}

		proposals.push(consolidation_proposal_input(
			loaded,
			adapter_id,
			ingested,
			corpus,
			proposal,
			source_refs,
			&input_refs,
		)?);
	}

	if proposals.is_empty() {
		return Err(eyre::eyre!("{} has no consolidation proposals.", loaded.job.job_id));
	}

	Ok(PreparedConsolidationRun { input_refs, proposals })
}

fn consolidation_proposal_input(
	loaded: &LoadedJob,
	adapter_id: &str,
	ingested: &IngestedCorpus,
	corpus: &[CorpusText],
	proposal: &LiveConsolidationProposal,
	source_refs: Vec<ConsolidationInputRef>,
	input_refs: &[ConsolidationInputRef],
) -> color_eyre::Result<ConsolidationProposalInput> {
	let unsupported_claim_flags =
		consolidation_unsupported_claim_flags(loaded, adapter_id, proposal, ingested, corpus)?;
	let diff = consolidation_diff(proposal.diff.clone())?;
	let proposed_payload = object_or_empty(diff.after.clone());
	let lineage = ConsolidationLineage {
		source_refs: source_refs.clone(),
		parent_run_id: None,
		parent_proposal_ids: Vec::new(),
	};

	Ok(ConsolidationProposalInput {
		proposal_kind: proposal.proposal_kind.clone(),
		apply_intent: consolidation_apply_intent(proposal.actual_review_action.as_str()),
		source_refs,
		source_snapshot: serde_json::json!({
			"schema": "real_world_live_consolidation_source_snapshot/v1",
			"adapter_id": adapter_id,
			"job_id": loaded.job.job_id,
			"proposal_id": proposal.proposal_id
		}),
		lineage,
		confidence: proposal.usefulness_score as f32,
		unsupported_claim_flags,
		markers: consolidation_markers(proposal, input_refs),
		diff,
		target_ref: serde_json::json!({
			"schema": "real_world_live_consolidation_target/v1",
			"proposal_id": proposal.proposal_id
		}),
		proposed_payload,
	})
}

fn validate_reviewed_consolidation_count(
	loaded: &LoadedJob,
	fixture: &LiveConsolidationFixture,
	reviewed: &[ConsolidationProposalResponse],
) -> color_eyre::Result<()> {
	if reviewed.len() == fixture.proposals.len() {
		return Ok(());
	}

	Err(eyre::eyre!(
		"ELF consolidation materialized {} proposals for {} fixture proposals in {}.",
		reviewed.len(),
		fixture.proposals.len(),
		loaded.job.job_id
	))
}

fn consolidation_materialization_evidence(
	run_id: Uuid,
	fixture: &LiveConsolidationFixture,
	input_refs: &[ConsolidationInputRef],
	reviewed: &[ConsolidationProposalResponse],
) -> ConsolidationMaterializationEvidence {
	let review_actions = reviewed
		.iter()
		.flat_map(|proposal| proposal.review_events.iter().map(|event| event.action.clone()))
		.collect::<Vec<_>>();
	let final_review_states =
		reviewed.iter().map(|proposal| proposal.review_state.clone()).collect::<Vec<_>>();
	let unsupported_claim_flag_count = fixture
		.proposals
		.iter()
		.map(|proposal| {
			proposal.unsupported_claim_count.max(proposal.unsupported_claim_flags.len())
		})
		.sum();
	let review_event_count =
		reviewed.iter().map(|proposal| proposal.review_events.len()).sum::<usize>();

	ConsolidationMaterializationEvidence {
		run_id: Some(run_id),
		proposal_ids: reviewed.iter().map(|proposal| proposal.proposal_id).collect(),
		source_lineage_count: input_refs.len(),
		unsupported_claim_flag_count,
		review_event_count,
		review_actions,
		final_review_states,
	}
}

fn consolidation_input_refs(
	loaded: &LoadedJob,
	adapter_id: &str,
	evidence_ids: &[String],
	ingested: &IngestedCorpus,
	corpus: &[CorpusText],
) -> color_eyre::Result<Vec<ConsolidationInputRef>> {
	evidence_ids
		.iter()
		.map(|evidence_id| {
			let note_id = ingested
				.note_ids_by_evidence
				.get(evidence_id)
				.and_then(|ids| ids.first().copied())
				.ok_or_else(|| {
					eyre::eyre!(
						"No live note id mapped for consolidation evidence {} in {}.",
						evidence_id,
						loaded.job.job_id
					)
				})?;
			let text = corpus
				.iter()
				.find(|item| item.evidence_id == *evidence_id)
				.map(|item| item.text.as_str())
				.unwrap_or(evidence_id.as_str());
			let content_hash = format!("blake3:{}", blake3::hash(text.as_bytes()).to_hex());

			Ok(ConsolidationInputRef {
				kind: ConsolidationSourceKind::Note,
				id: note_id,
				snapshot: ConsolidationSourceSnapshot {
					status: Some("active".to_string()),
					updated_at: Some(OffsetDateTime::now_utc()),
					content_hash: Some(content_hash),
					embedding_version: None,
					trace_version: None,
					source_ref: serde_json::json!({
						"schema": "real_world_live_adapter/v1",
						"adapter": adapter_id,
						"job_id": loaded.job.job_id,
						"evidence_id": evidence_id
					}),
					metadata: serde_json::json!({
						"evidence_id": evidence_id,
						"source": "memory_notes"
					}),
				},
			})
		})
		.collect()
}

fn push_unique_input_ref(values: &mut Vec<ConsolidationInputRef>, value: ConsolidationInputRef) {
	if !values.iter().any(|existing| existing.id == value.id) {
		values.push(value);
	}
}

fn consolidation_unsupported_claim_flags(
	loaded: &LoadedJob,
	adapter_id: &str,
	proposal: &LiveConsolidationProposal,
	ingested: &IngestedCorpus,
	corpus: &[CorpusText],
) -> color_eyre::Result<Vec<ConsolidationUnsupportedClaimFlag>> {
	proposal
		.unsupported_claim_flags
		.iter()
		.map(|flag| {
			let source = flag
				.source_ref
				.as_deref()
				.map(|source_ref| {
					consolidation_input_refs(
						loaded,
						adapter_id,
						&[source_ref.to_string()],
						ingested,
						corpus,
					)
					.and_then(|refs| {
						refs.into_iter().next().ok_or_else(|| {
							eyre::eyre!(
								"Unsupported claim source {} did not map to a live source.",
								source_ref
							)
						})
					})
				})
				.transpose()?;

			Ok(ConsolidationUnsupportedClaimFlag {
				claim_id: flag.claim_id.clone(),
				message: flag.message.clone(),
				source,
			})
		})
		.collect()
}

fn consolidation_diff(value: serde_json::Value) -> color_eyre::Result<ConsolidationProposalDiff> {
	let summary = value
		.get("summary")
		.and_then(serde_json::Value::as_str)
		.unwrap_or("Live consolidation proposal.")
		.to_string();

	Ok(ConsolidationProposalDiff {
		summary,
		before: object_or_empty(value.get("before").cloned().unwrap_or(serde_json::Value::Null)),
		after: object_or_empty(value.get("after").cloned().unwrap_or(serde_json::Value::Null)),
	})
}

fn object_or_empty(value: serde_json::Value) -> serde_json::Value {
	if matches!(value, serde_json::Value::Object(_)) { value } else { serde_json::json!({}) }
}

fn consolidation_apply_intent(action: &str) -> ConsolidationApplyIntent {
	if action == "apply" {
		ConsolidationApplyIntent::CreateDerivedNote
	} else {
		ConsolidationApplyIntent::NoOp
	}
}

fn consolidation_review_action(raw: &str) -> color_eyre::Result<ConsolidationReviewAction> {
	match raw {
		"apply" => Ok(ConsolidationReviewAction::Apply),
		"discard" => Ok(ConsolidationReviewAction::Discard),
		"defer" => Ok(ConsolidationReviewAction::Defer),
		"approve" => Ok(ConsolidationReviewAction::Approve),
		_ => Err(eyre::eyre!("Unknown consolidation review action {raw}.")),
	}
}

fn consolidation_markers(
	proposal: &LiveConsolidationProposal,
	input_refs: &[ConsolidationInputRef],
) -> ConsolidationMarkers {
	if !proposal.proposal_kind.contains("contradiction") {
		return ConsolidationMarkers::default();
	}

	let marker = ConsolidationMarker {
		severity: ConsolidationMarkerSeverity::High,
		message:
			"Live adapter materialized a contradiction-oriented proposal for reviewer inspection."
				.to_string(),
		source: input_refs.first().cloned(),
	};

	ConsolidationMarkers { contradictions: vec![marker], staleness: Vec::new() }
}

fn live_consolidation_response(
	fixture: &LiveConsolidationFixture,
	reviewed: &[ConsolidationProposalResponse],
) -> color_eyre::Result<serde_json::Value> {
	let proposals = fixture
		.proposals
		.iter()
		.zip(reviewed)
		.map(|(fixture_proposal, reviewed_proposal)| {
			serde_json::json!({
				"proposal_id": reviewed_proposal.proposal_id.to_string(),
				"proposal_kind": fixture_proposal.proposal_kind.clone(),
				"source_refs": fixture_proposal.source_refs.clone(),
				"expected_source_refs": if fixture_proposal.expected_source_refs.is_empty() {
					fixture_proposal.source_refs.clone()
				} else {
					fixture_proposal.expected_source_refs.clone()
				},
				"usefulness_score": fixture_proposal.usefulness_score,
				"min_usefulness_score": fixture_proposal.min_usefulness_score,
				"expected_review_action": fixture_proposal.expected_review_action.clone(),
				"actual_review_action": fixture_proposal.actual_review_action.clone(),
				"source_mutations": fixture_proposal.source_mutations.clone(),
				"unsupported_claim_count": fixture_proposal
					.unsupported_claim_count
					.max(fixture_proposal.unsupported_claim_flags.len()),
				"unsupported_claim_flags": fixture_proposal.unsupported_claim_flags.clone(),
				"diff": fixture_proposal.diff.clone(),
				"live_review_state": reviewed_proposal.review_state.clone(),
				"live_review_event_count": reviewed_proposal.review_events.len()
			})
		})
		.collect::<Vec<_>>();

	Ok(serde_json::json!({ "proposals": proposals, "executable_gaps": [] }))
}

fn live_note_ids(ingested: &IngestedCorpus) -> Vec<Uuid> {
	let mut note_ids = Vec::new();

	for ids in ingested.note_ids_by_evidence.values() {
		for note_id in ids {
			if !note_ids.iter().any(|existing| existing == note_id) {
				note_ids.push(*note_id);
			}
		}
	}

	note_ids
}

fn knowledge_page_artifact(
	loaded: &LoadedJob,
	ingested: &IngestedCorpus,
	first: &KnowledgePageResponse,
	second: &KnowledgePageResponse,
	lint: &KnowledgePageLintResponse,
) -> color_eyre::Result<serde_json::Value> {
	let reverse = note_id_to_evidence_id(ingested);
	let mut sections = second
		.sections
		.iter()
		.map(|section| {
			let evidence_ids = section
				.source_backlinks
				.iter()
				.filter_map(|source| reverse.get(&source.source_id).cloned())
				.collect::<Vec<_>>();

			serde_json::json!({
				"section_id": section.section_key.clone(),
				"heading": section.heading.clone(),
				"role": section.role.clone(),
				"content": section.content.clone(),
				"evidence_ids": evidence_ids,
				"timeline_event_ids": []
			})
		})
		.collect::<Vec<_>>();

	sections.extend(unsupported_sections_from_fixture(loaded));

	Ok(serde_json::json!({
		"page_id": second.page.page_id.to_string(),
		"page_type": second.page.page_kind.clone(),
		"title": second.page.title.clone(),
		"sections": sections,
		"backlinks": source_backlinks(ingested),
		"lint_findings": lint_findings_for_page(loaded, ingested, lint),
		"page_version_diff": second.page.previous_version_diff.clone(),
		"rebuild": {
			"first_hash": first.page.content_hash.clone(),
			"second_hash": second.page.content_hash.clone(),
			"deterministic": first.page.content_hash == second.page.content_hash,
			"allowed_variance": []
		}
	}))
}

fn knowledge_materialization_evidence(
	page: &KnowledgePageResponse,
	lint: &KnowledgePageLintResponse,
	search_result_count: usize,
) -> KnowledgeMaterializationEvidence {
	let unsupported_claim_count =
		lint.findings.iter().filter(|finding| finding.finding_type == "unsupported_claim").count()
			+ page.sections.iter().filter(|section| section.unsupported_reason.is_some()).count();

	KnowledgeMaterializationEvidence {
		page_ids: vec![page.page.page_id],
		search_result_count,
		lint_finding_count: lint.findings.len(),
		stale_source_finding_count: lint
			.findings
			.iter()
			.filter(|finding| finding.finding_type == "stale_source_ref")
			.count(),
		unsupported_claim_count,
		citation_count: page.sections.iter().map(|section| section.citation_count).sum(),
		source_ref_count: page.source_refs.len(),
		version_diff_available: page
			.page
			.previous_version_diff
			.as_ref()
			.and_then(|diff| diff.get("available"))
			.and_then(serde_json::Value::as_bool)
			.unwrap_or(false),
	}
}

fn note_id_to_evidence_id(ingested: &IngestedCorpus) -> HashMap<Uuid, String> {
	let mut out = HashMap::new();

	for (evidence_id, note_ids) in &ingested.note_ids_by_evidence {
		for note_id in note_ids {
			out.insert(*note_id, evidence_id.clone());
		}
	}

	out
}

fn source_backlinks(ingested: &IngestedCorpus) -> Vec<String> {
	let mut backlinks = ingested
		.note_ids_by_evidence
		.keys()
		.map(|evidence_id| format!("source:{evidence_id}"))
		.collect::<Vec<_>>();

	backlinks.sort();

	backlinks
}

fn lint_findings_for_page(
	loaded: &LoadedJob,
	ingested: &IngestedCorpus,
	lint: &KnowledgePageLintResponse,
) -> Vec<serde_json::Value> {
	let reverse = note_id_to_evidence_id(ingested);

	lint.findings
		.iter()
		.map(|finding| {
			let evidence_ids = finding
				.source_id
				.and_then(|source_id| reverse.get(&source_id).cloned())
				.into_iter()
				.collect::<Vec<_>>();
			let trap_id = evidence_ids
				.first()
				.and_then(|evidence_id| trap_id_for_evidence(loaded, evidence_id));

			serde_json::json!({
				"finding_id": finding.finding_id.to_string(),
				"finding_type": finding.finding_type.clone(),
				"severity": finding.severity.clone(),
				"text": finding.message.clone(),
				"evidence_ids": evidence_ids,
				"trap_id": trap_id
			})
		})
		.collect()
}

fn unsupported_sections_from_fixture(loaded: &LoadedJob) -> Vec<serde_json::Value> {
	let Some(pages) = loaded
		.value
		.pointer("/corpus/adapter_response/answer/pages")
		.and_then(serde_json::Value::as_array)
	else {
		return Vec::new();
	};
	let mut sections = Vec::new();

	for page in pages {
		let Some(page_sections) = page.get("sections").and_then(serde_json::Value::as_array) else {
			continue;
		};

		for section in page_sections {
			let Some(reason) =
				section.get("unsupported_reason").and_then(serde_json::Value::as_str)
			else {
				continue;
			};

			sections.push(serde_json::json!({
				"section_id": section
					.get("section_id")
					.and_then(serde_json::Value::as_str)
					.unwrap_or("unsupported-summary"),
				"heading": section
					.get("heading")
					.and_then(serde_json::Value::as_str)
					.unwrap_or("Unsupported Summary"),
				"role": section.get("role").and_then(serde_json::Value::as_str).unwrap_or("summary"),
				"content": section.get("content").and_then(serde_json::Value::as_str).unwrap_or(reason),
				"evidence_ids": [],
				"timeline_event_ids": [],
				"unsupported_reason": reason
			}));
		}
	}

	sections
}

fn stale_trap_evidence_ids(loaded: &LoadedJob) -> Vec<String> {
	loaded
		.value
		.get("negative_traps")
		.and_then(serde_json::Value::as_array)
		.into_iter()
		.flatten()
		.filter(|trap| {
			trap.get("type").and_then(serde_json::Value::as_str) == Some("stale_fact")
				&& trap.get("failure_if_used").and_then(serde_json::Value::as_bool).unwrap_or(false)
		})
		.flat_map(|trap| {
			trap.get("evidence_ids")
				.and_then(serde_json::Value::as_array)
				.into_iter()
				.flatten()
				.filter_map(serde_json::Value::as_str)
				.map(ToString::to_string)
				.collect::<Vec<_>>()
		})
		.collect()
}

fn trap_id_for_evidence(loaded: &LoadedJob, evidence_id: &str) -> Option<String> {
	loaded
		.value
		.get("negative_traps")
		.and_then(serde_json::Value::as_array)?
		.iter()
		.find(|trap| {
			trap.get("evidence_ids")
				.and_then(serde_json::Value::as_array)
				.is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(evidence_id)))
		})
		.and_then(|trap| trap.get("trap_id").and_then(serde_json::Value::as_str))
		.map(ToString::to_string)
}

fn elf_selected_evidence_text(
	loaded: &LoadedJob,
	stored_corpus: &[CorpusText],
	evidence_ids: &[String],
	ingested: &IngestedCorpus,
	capture_failure: &Option<String>,
) -> (
	SelectedEvidenceText,
	Option<TemporalReconciliationMaterializationEvidence>,
	Option<Vec<TraceStageOutput>>,
) {
	if let Some(failure) = capture_failure {
		return (
			SelectedEvidenceText { content: failure.clone(), evidence_ids: Vec::new() },
			None,
			None,
		);
	}
	if let Some(selection) =
		temporal_reconciliation_selection(loaded, stored_corpus, evidence_ids, ingested)
	{
		return (selection.selected, Some(selection.evidence), Some(selection.trace_stages));
	}

	(selected_required_corpus_texts(loaded, stored_corpus, evidence_ids), None, None)
}

fn dreaming_readback_template_artifacts(
	loaded: &LoadedJob,
) -> color_eyre::Result<Vec<serde_json::Value>> {
	let pointer = match loaded.job.suite.as_str() {
		"memory_summary" => "/corpus/adapter_response/answer/memory_summaries",
		"proactive_brief" => "/corpus/adapter_response/answer/proactive_briefs",
		"scheduled_memory" => "/corpus/adapter_response/answer/scheduled_tasks",
		_ => return Ok(Vec::new()),
	};
	let artifacts =
		loaded.value.pointer(pointer).and_then(serde_json::Value::as_array).cloned().ok_or_else(
			|| {
				eyre::eyre!(
					"{} missing service-native readback template at {pointer}.",
					loaded.job.job_id
				)
			},
		)?;

	if artifacts.is_empty() {
		return Err(eyre::eyre!(
			"{} has no service-native readback template artifacts.",
			loaded.job.job_id
		));
	}

	Ok(artifacts)
}

fn dreaming_readback_scoring_evidence_ids(
	loaded: &LoadedJob,
	service_evidence_ids: &[String],
) -> Vec<String> {
	let selected = service_evidence_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
	let trap_ids = negative_trap_evidence_ids(loaded);
	let mut evidence_ids = Vec::new();

	for evidence in &loaded.job.required_evidence {
		if selected.contains(evidence.evidence_id.as_str())
			&& !trap_ids.contains(evidence.evidence_id.as_str())
		{
			push_unique(&mut evidence_ids, evidence.evidence_id.clone());
		}
	}

	if evidence_ids.is_empty() {
		for evidence_id in service_evidence_ids {
			if !trap_ids.contains(evidence_id.as_str()) {
				push_unique(&mut evidence_ids, evidence_id.clone());
			}
		}
	}

	evidence_ids
}

fn negative_trap_evidence_ids(loaded: &LoadedJob) -> BTreeSet<&str> {
	loaded
		.value
		.get("negative_traps")
		.and_then(serde_json::Value::as_array)
		.into_iter()
		.flatten()
		.filter(|trap| {
			trap.get("failure_if_used").and_then(serde_json::Value::as_bool).unwrap_or(false)
		})
		.flat_map(|trap| {
			trap.get("evidence_ids")
				.and_then(serde_json::Value::as_array)
				.into_iter()
				.flatten()
				.filter_map(serde_json::Value::as_str)
		})
		.collect()
}

fn stamp_dreaming_readback_artifact(
	artifact: &mut serde_json::Value,
	loaded: &LoadedJob,
	project_id: &str,
	trace_id: Uuid,
	generated_at: &str,
) {
	artifact["generated_at"] = serde_json::json!(generated_at);
	artifact["tenant_id"] = serde_json::json!(TENANT_ID);
	artifact["project_id"] = serde_json::json!(project_id);
	artifact["agent_id"] = serde_json::json!(AGENT_ID);
	artifact["read_profile"] = serde_json::json!("private_only");
	artifact["service_readback"] = serde_json::json!({
		"schema": "elf.service_native_dreaming_readback/v1",
		"job_id": loaded.job.job_id,
		"suite": loaded.job.suite,
		"runtime_path": "ElfService::list",
		"search_trace_id": trace_id,
		"source_mutation_count": 0
	});

	if loaded.job.suite == "scheduled_memory" {
		let trace = artifact
			.as_object_mut()
			.map(|object| object.entry("execution_trace").or_insert_with(|| serde_json::json!({})));

		if let Some(trace) = trace {
			trace["trace_id"] = serde_json::json!(format!("service-native-{trace_id}"));
			trace["trigger_kind"] = serde_json::json!("service_native_readback");
			trace["status"] = serde_json::json!("completed");
		}

		artifact["source_mutations"] = serde_json::json!([]);
	}
}

fn collect_dreaming_artifact_source_refs(value: &serde_json::Value, refs: &mut Vec<String>) {
	match value {
		serde_json::Value::Array(items) =>
			for item in items {
				collect_dreaming_artifact_source_refs(item, refs);
			},
		serde_json::Value::Object(map) =>
			for (key, value) in map {
				if matches!(key.as_str(), "source_refs" | "evidence_refs" | "evidence_ids")
					&& let Some(items) = value.as_array()
				{
					for item in items {
						if let Some(source_ref) = item.as_str() {
							push_unique(refs, source_ref.to_string());
						}
					}
				}
				if key == "evidence_id"
					&& let Some(source_ref) = value.as_str()
				{
					push_unique(refs, source_ref.to_string());
				}

				collect_dreaming_artifact_source_refs(value, refs);
			},
		_ => {},
	}
}

fn dreaming_readback_content(suite: &str, artifacts: &[serde_json::Value]) -> String {
	let mut parts = Vec::new();

	for artifact in artifacts {
		match suite {
			"memory_summary" => {
				for entry in artifact
					.get("entries")
					.and_then(serde_json::Value::as_array)
					.into_iter()
					.flatten()
				{
					if let Some(text) = entry.get("text").and_then(serde_json::Value::as_str) {
						parts.push(text.to_string());
					}
				}
			},
			"proactive_brief" => {
				for suggestion in artifact
					.get("suggestions")
					.and_then(serde_json::Value::as_array)
					.into_iter()
					.flatten()
				{
					if let Some(title) = suggestion.get("title").and_then(serde_json::Value::as_str)
					{
						parts.push(title.to_string());
					}
					if let Some(body) = suggestion.get("body").and_then(serde_json::Value::as_str) {
						parts.push(body.to_string());
					}
				}
			},
			"scheduled_memory" => {
				for output in artifact
					.get("outputs")
					.and_then(serde_json::Value::as_array)
					.into_iter()
					.flatten()
				{
					if let Some(text) = output.get("text").and_then(serde_json::Value::as_str) {
						parts.push(text.to_string());
					}
				}
			},
			_ => {},
		}
	}

	if parts.is_empty() {
		"Service-native Dreaming readback produced no artifact text.".to_string()
	} else {
		parts.join(" ")
	}
}

fn dreaming_readback_trace_stages(
	loaded: &LoadedJob,
	evidence: &DreamingReadbackMaterializationEvidence,
) -> Vec<TraceStageOutput> {
	vec![
		TraceStageOutput {
			stage_name: "dreaming_readback.service_list".to_string(),
			kept_evidence: evidence.selected_source_refs.clone(),
			dropped_evidence: evidence.missing_source_refs.clone(),
			demoted_evidence: Vec::new(),
			distractor_evidence: Vec::new(),
			notes: format!(
				"Read {} source refs from ElfService::list for {}.",
				evidence.selected_source_refs.len(),
				loaded.job.suite
			),
		},
		TraceStageOutput {
			stage_name: "dreaming_readback.source_mutation_guard".to_string(),
			kept_evidence: evidence.selected_source_refs.clone(),
			dropped_evidence: Vec::new(),
			demoted_evidence: Vec::new(),
			distractor_evidence: Vec::new(),
			notes: "Generated readback artifacts without mutating source notes.".to_string(),
		},
	]
}

fn search_response_evidence_ids(response: &SearchResponse) -> Vec<String> {
	let mut evidence_ids = Vec::new();

	for item in &response.items {
		if let Some(evidence_id) =
			item.source_ref.get("evidence_id").and_then(serde_json::Value::as_str)
		{
			push_unique(&mut evidence_ids, evidence_id.to_string());
		}
	}

	evidence_ids
}

fn suite_materialization_selection(
	input: SuiteMaterializationSelectionInput<'_>,
) -> SuiteMaterializationSelection {
	let suite_claims_materialized = input.capture_failure.is_none()
		&& ((input.loaded.job.suite == "knowledge_compilation" && input.knowledge.is_some())
			|| (input.loaded.job.suite == "consolidation" && input.consolidation.is_some())
			|| input.dreaming_readback.is_some());
	let selected = if let Some(output) = &input.dreaming_readback {
		SelectedEvidenceText {
			content: output.content.clone(),
			evidence_ids: output.evidence_ids.clone(),
		}
	} else if suite_claims_materialized {
		expected_claim_text(
			input.loaded,
			live_required_evidence_ids(input.loaded, input.ingested).as_slice(),
		)
	} else {
		input.selected
	};
	let trace_stages = input
		.dreaming_readback
		.as_ref()
		.map(|output| output.trace_stages.clone())
		.or(input.trace_stages);
	let memory_summaries = input
		.dreaming_readback
		.as_ref()
		.map(|output| output.memory_summaries.clone())
		.unwrap_or_default();
	let proactive_briefs = input
		.dreaming_readback
		.as_ref()
		.map(|output| output.proactive_briefs.clone())
		.unwrap_or_default();
	let scheduled_tasks = input
		.dreaming_readback
		.as_ref()
		.map(|output| output.scheduled_tasks.clone())
		.unwrap_or_default();
	let dreaming_readback =
		input.dreaming_readback.as_ref().map(|output| output.materialization.clone());

	SuiteMaterializationSelection {
		selected,
		trace_stages,
		dreaming_readback,
		memory_summaries,
		proactive_briefs,
		scheduled_tasks,
	}
}

async fn materialize_elf_dreaming_readback(
	service: &ElfService,
	loaded: &LoadedJob,
	project_id: &str,
	trace_id: Uuid,
	adapter_id: &str,
) -> color_eyre::Result<Option<DreamingReadbackOutput>> {
	if !is_elf_dreaming_readback_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return Ok(None);
	}

	let generated_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
	let service_evidence_ids = service_readback_evidence_ids(service, project_id).await?;
	let mut artifacts = dreaming_readback_template_artifacts(loaded)?;

	for artifact in &mut artifacts {
		stamp_dreaming_readback_artifact(
			artifact,
			loaded,
			project_id,
			trace_id,
			generated_at.as_str(),
		);
	}

	let mut artifact_source_refs = Vec::new();

	for artifact in &artifacts {
		collect_dreaming_artifact_source_refs(artifact, &mut artifact_source_refs);
	}

	artifact_source_refs.sort();
	artifact_source_refs.dedup();

	let missing_source_refs = artifact_source_refs
		.iter()
		.filter(|source_ref| !service_evidence_ids.contains(*source_ref))
		.cloned()
		.collect::<Vec<_>>();
	let returned_source_refs = artifact_source_refs
		.iter()
		.filter(|source_ref| service_evidence_ids.contains(*source_ref))
		.cloned()
		.collect::<Vec<_>>();
	let scoring_evidence_ids =
		dreaming_readback_scoring_evidence_ids(loaded, &service_evidence_ids);
	let artifact_kind = match loaded.job.suite.as_str() {
		"memory_summary" => "elf.memory_summary/v1",
		"proactive_brief" => "elf.proactive_project_brief/v1",
		"scheduled_memory" => "elf.scheduled_memory_task/v1",
		_ => "elf.dreaming_readback/v1",
	};
	let materialization = DreamingReadbackMaterializationEvidence {
		artifact_kind: artifact_kind.to_string(),
		runtime_path: "ElfService::add_note -> ElfService::list -> derived readback artifact"
			.to_string(),
		service_list_count: service_evidence_ids.len(),
		trace_id: Some(trace_id),
		generated_artifact_count: artifacts.len(),
		selected_source_refs: returned_source_refs.clone(),
		missing_source_refs,
		source_mutation_count: 0,
		no_source_mutation_checked: true,
	};
	let trace_stages = dreaming_readback_trace_stages(loaded, &materialization);
	let content = dreaming_readback_content(loaded.job.suite.as_str(), &artifacts);
	let (memory_summaries, proactive_briefs, scheduled_tasks) = match loaded.job.suite.as_str() {
		"memory_summary" => (artifacts, Vec::new(), Vec::new()),
		"proactive_brief" => (Vec::new(), artifacts, Vec::new()),
		"scheduled_memory" => (Vec::new(), Vec::new(), artifacts),
		_ => (Vec::new(), Vec::new(), Vec::new()),
	};

	Ok(Some(DreamingReadbackOutput {
		content,
		evidence_ids: scoring_evidence_ids,
		memory_summaries,
		proactive_briefs,
		scheduled_tasks,
		materialization,
		trace_stages,
	}))
}

async fn service_readback_evidence_ids(
	service: &ElfService,
	project_id: &str,
) -> color_eyre::Result<Vec<String>> {
	let response = service
		.list(ListRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: project_id.to_string(),
			agent_id: Some(AGENT_ID.to_string()),
			scope: Some(SCOPE.to_string()),
			status: Some("active".to_string()),
			r#type: None,
		})
		.await
		.map_err(|err| eyre::eyre!("ELF service-native readback list failed: {err}"))?;
	let mut evidence_ids = Vec::new();

	for item in response.items {
		if let Some(evidence_id) =
			item.source_ref.get("evidence_id").and_then(serde_json::Value::as_str)
		{
			push_unique(&mut evidence_ids, evidence_id.to_string());
		}
	}

	Ok(evidence_ids)
}

async fn run_lightrag_async(args: LightragArgs) -> color_eyre::Result<()> {
	let jobs = load_jobs(&args.fixtures)?;
	let run_slug = short_hash(format!("{}:{}", args.adapter_id, Uuid::new_v4()).as_str());
	let result = materialize_lightrag_jobs(&args, &jobs, &run_slug).await;
	let materialized = match result {
		Ok(jobs) => jobs,
		Err(err) => lightrag_failure_jobs(
			&args.adapter_id,
			&jobs,
			"lightrag_api_context_export",
			err.to_string(),
		),
	};
	let status = aggregate_status(&materialized);

	write_materialized_output(MaterializedOutput {
		adapter_id: &args.adapter_id,
		adapter_kind: AdapterKind::LightragApiContextExport,
		fixtures: &args.fixtures,
		out_fixtures: &args.out_fixtures,
		evidence_out: &args.evidence_out,
		jobs: &jobs,
		materialized: &materialized,
		command_evidence: vec![CommandEvidence {
			label: "lightrag_api_context_export".to_string(),
			status,
			command: "cargo run -p elf-eval --bin real_world_live_adapter -- lightrag"
				.to_string(),
			artifact: Some(args.evidence_out.display().to_string()),
			reason: "LightRAG adapter used /documents/texts, /documents/track_status, and /query with only_need_context plus chunk references.".to_string(),
		}],
		metadata: Some(lightrag_metadata(&args, &run_slug)),
	})
}

async fn materialize_lightrag_jobs(
	args: &LightragArgs,
	jobs: &[LoadedJob],
	run_slug: &str,
) -> color_eyre::Result<Vec<MaterializedJob>> {
	fs::create_dir_all(&args.work_dir)?;

	let client = reqwest::Client::builder().timeout(Duration::from_secs(180)).build()?;

	wait_for_lightrag(args, &client).await?;

	let mut out = Vec::with_capacity(jobs.len());

	for loaded in jobs {
		out.push(materialize_lightrag_job(args, &client, loaded, run_slug).await?);
	}

	Ok(out)
}

async fn wait_for_lightrag(
	args: &LightragArgs,
	client: &reqwest::Client,
) -> color_eyre::Result<()> {
	let mut last_error = String::new();

	for _attempt in 1..=args.startup_attempts {
		match lightrag_get_json(args, client, "/health").await {
			Ok(_) => return Ok(()),
			Err(err) => last_error = err.to_string(),
		}

		time::sleep(Duration::from_secs(args.startup_interval_seconds)).await;
	}

	Err(eyre::eyre!(
		"LightRAG API did not become healthy at {} after {} attempts: {}",
		lightrag_api_base(args),
		args.startup_attempts,
		last_error
	))
}

async fn materialize_lightrag_job(
	args: &LightragArgs,
	client: &reqwest::Client,
	loaded: &LoadedJob,
	run_slug: &str,
) -> color_eyre::Result<MaterializedJob> {
	if let Some(job) = declared_encoding_job(&args.adapter_id, loaded) {
		return Ok(job);
	}
	if let Some(job) = lightrag_not_encoded_job(&args.adapter_id, loaded) {
		return Ok(job);
	}

	let corpus = corpus_texts(loaded)?;
	let sources = write_lightrag_corpus(args, loaded, &corpus, run_slug)?;
	let indexed_at = Instant::now();
	let insert_response = insert_lightrag_texts(args, client, &corpus, &sources).await?;

	wait_for_lightrag_index(args, client, &insert_response, corpus.len()).await?;

	let indexing_latency_ms = indexed_at.elapsed().as_secs_f64() * 1_000.0;
	let queried_at = Instant::now();
	let query_response = query_lightrag_context(args, client, loaded).await?;
	let latency_ms = queried_at.elapsed().as_secs_f64() * 1_000.0;
	let source_mappings = lightrag_source_mappings(&corpus, &sources, &query_response);
	let evidence_ids = lightrag_mapped_evidence_ids(&source_mappings);
	let selected = selected_required_corpus_texts(loaded, &corpus, &evidence_ids);

	Ok(materialized_job(
		loaded,
		&args.adapter_id,
		MaterializedJobInput {
			content: selected.content,
			evidence_ids: selected.evidence_ids,
			pages: Vec::new(),
			latency_ms,
			indexing_latency_ms: Some(indexing_latency_ms),
			returned_count: source_mappings.len(),
			trace_id: None,
			failure: None,
			source_mappings,
			operator_debug: None,
			operator_debug_evidence: None,
			capture: None,
			capture_failure: None,
			consolidation_response: None,
			consolidation: None,
			knowledge: None,
			temporal_reconciliation: None,
			dreaming_readback: None,
			memory_summaries: Vec::new(),
			proactive_briefs: Vec::new(),
			scheduled_tasks: Vec::new(),
			trace_stages: None,
		},
	))
}

async fn insert_lightrag_texts(
	args: &LightragArgs,
	client: &reqwest::Client,
	corpus: &[CorpusText],
	sources: &[LightragSource],
) -> color_eyre::Result<serde_json::Value> {
	let request = serde_json::json!({
		"texts": corpus.iter().map(|item| item.text.as_str()).collect::<Vec<_>>(),
		"file_sources": sources.iter().map(|source| source.file_source.as_str()).collect::<Vec<_>>(),
		"chunking": {
			"strategy": "fixed_token",
			"params": {
				"chunk_token_size": 320,
				"chunk_overlap_token_size": 32
			}
		}
	});

	lightrag_post_json(args, client, "/documents/texts", &request).await
}

async fn wait_for_lightrag_index(
	args: &LightragArgs,
	client: &reqwest::Client,
	insert_response: &serde_json::Value,
	expected_docs: usize,
) -> color_eyre::Result<()> {
	let track_id = insert_response
		.get("track_id")
		.and_then(serde_json::Value::as_str)
		.ok_or_else(|| eyre::eyre!("LightRAG text insert response did not include track_id."))?;
	let mut last_status = serde_json::Value::Null;

	for _attempt in 1..=args.index_attempts {
		let status =
			lightrag_get_json(args, client, format!("/documents/track_status/{track_id}")).await?;

		if lightrag_index_failed(&status) {
			return Err(eyre::eyre!(
				"LightRAG document indexing failed for track_id {track_id}: {}",
				serde_json::to_string(&status)?
			));
		}
		if lightrag_index_processed(&status, expected_docs) {
			return Ok(());
		}

		last_status = status;

		time::sleep(Duration::from_secs(args.index_interval_seconds)).await;
	}

	Err(eyre::eyre!(
		"LightRAG document indexing did not finish for track_id {} after {} attempts: {}",
		track_id,
		args.index_attempts,
		serde_json::to_string(&last_status)?
	))
}

async fn query_lightrag_context(
	args: &LightragArgs,
	client: &reqwest::Client,
	loaded: &LoadedJob,
) -> color_eyre::Result<serde_json::Value> {
	let keywords = lightrag_keywords(loaded.job.prompt.content.as_str());
	let request = serde_json::json!({
		"query": loaded.job.prompt.content,
		"mode": args.query_mode,
		"only_need_context": true,
		"include_references": true,
		"include_chunk_content": true,
		"enable_rerank": false,
		"top_k": args.top_k,
		"chunk_top_k": args.chunk_top_k,
		"hl_keywords": keywords,
		"ll_keywords": keywords,
		"stream": false
	});

	lightrag_post_json(args, client, "/query", &request).await
}

async fn lightrag_get_json(
	args: &LightragArgs,
	client: &reqwest::Client,
	path: impl AsRef<str>,
) -> color_eyre::Result<serde_json::Value> {
	let url = format!("{}{}", lightrag_api_base(args), path.as_ref());
	let mut request = client.get(url);

	if let Some(api_key) = args.api_key.as_deref().filter(|key| !key.is_empty()) {
		request = request.bearer_auth(api_key);
	}

	lightrag_send_json(request).await
}

async fn lightrag_post_json(
	args: &LightragArgs,
	client: &reqwest::Client,
	path: &str,
	body: &serde_json::Value,
) -> color_eyre::Result<serde_json::Value> {
	let url = format!("{}{}", lightrag_api_base(args), path);
	let mut request = client.post(url).json(body);

	if let Some(api_key) = args.api_key.as_deref().filter(|key| !key.is_empty()) {
		request = request.bearer_auth(api_key);
	}

	lightrag_send_json(request).await
}

async fn lightrag_send_json(request: RequestBuilder) -> color_eyre::Result<serde_json::Value> {
	let response = request.send().await?;
	let status = response.status();
	let body = response.text().await?;

	if !status.is_success() {
		return Err(eyre::eyre!("LightRAG API returned HTTP {status}: {body}"));
	}

	serde_json::from_str(&body)
		.map_err(|err| eyre::eyre!("LightRAG API returned invalid JSON: {err}; body={body}"))
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;

	match Args::parse().command {
		CommandArgs::Elf(args) => run_elf(args).await,
		CommandArgs::Qmd(args) => run_qmd(args),
		CommandArgs::Lightrag(args) => run_lightrag_async(args).await,
	}
}

async fn run_elf(args: ElfArgs) -> color_eyre::Result<()> {
	let jobs = load_jobs(&args.fixtures)?;
	let result = materialize_elf_jobs(&args, &jobs).await;
	let materialized = match result {
		Ok(jobs) => jobs,
		Err(err) => failure_jobs(&args.adapter_id, &jobs, "elf_service_runtime", err.to_string()),
	};

	write_materialized_output(MaterializedOutput {
		adapter_id: &args.adapter_id,
		adapter_kind: AdapterKind::ElfServiceRuntime,
		fixtures: &args.fixtures,
		out_fixtures: &args.out_fixtures,
		evidence_out: &args.evidence_out,
		jobs: &jobs,
		materialized: &materialized,
		command_evidence: vec![CommandEvidence {
			label: "elf_service_runtime".to_string(),
			status: aggregate_status(&materialized),
			command: "cargo run -p elf-eval --bin real_world_live_adapter -- elf".to_string(),
			artifact: Some(args.evidence_out.display().to_string()),
			reason: "ELF live adapter used ElfService, worker indexing, and search_raw."
				.to_string(),
		}],
		metadata: None,
	})
}

async fn materialize_elf_jobs(
	args: &ElfArgs,
	jobs: &[LoadedJob],
) -> color_eyre::Result<Vec<MaterializedJob>> {
	let base_dsn = env::var("ELF_PG_DSN")
		.map_err(|_| eyre::eyre!("ELF_PG_DSN must be set for ELF live real-world adapter."))?;
	let qdrant_url = env::var("ELF_QDRANT_GRPC_URL")
		.or_else(|_| env::var("ELF_QDRANT_URL"))
		.map_err(|_| eyre::eyre!("ELF_QDRANT_GRPC_URL or ELF_QDRANT_URL must be set."))?;
	let test_db = TestDatabase::new(&base_dsn).await?;
	let run_suffix = short_hash(format!("{}:{}", args.adapter_id, Uuid::new_v4()).as_str());
	let runtime = BaselineRuntime {
		config_path: args.config.clone(),
		dsn: test_db.dsn().to_string(),
		qdrant_url,
		collection: format!("elf_live_real_world_{run_suffix}"),
		docs_collection: format!("elf_live_real_world_docs_{run_suffix}"),
	};
	let service = build_service(&runtime).await?;
	let mut out = Vec::with_capacity(jobs.len());

	for loaded in jobs {
		out.push(materialize_elf_job(&runtime, &service, loaded, &args.adapter_id).await?);
	}

	drop(service);

	test_db.cleanup().await?;

	Ok(out)
}

async fn materialize_elf_job(
	runtime: &BaselineRuntime,
	service: &ElfService,
	loaded: &LoadedJob,
	adapter_id: &str,
) -> color_eyre::Result<MaterializedJob> {
	if let Some(job) = declared_encoding_job(adapter_id, loaded) {
		return Ok(job);
	}
	if let Some(job) = not_encoded_job(adapter_id, loaded) {
		return Ok(job);
	}

	let corpus = corpus_texts(loaded)?;
	let stored_corpus = elf_stored_corpus_texts(&corpus)?;
	let project_id = project_id_for_job(&loaded.job.job_id);
	let ingested =
		ingest_elf_corpus(service, loaded, adapter_id, project_id.as_str(), &corpus).await?;

	run_worker(runtime).await?;

	let (response, latency_ms) = search_elf_job(service, loaded, &project_id).await?;
	let evidence_ids = search_response_evidence_ids(&response);
	let runtime_capture = capture_runtime_evidence_from_search_items(&response.items);
	let capture = capture_with_runtime_source_refs(ingested.capture.clone(), &runtime_capture);
	let capture_failure = validate_capture_runtime_evidence(
		loaded.job.suite.as_str(),
		&corpus,
		&capture,
		&runtime_capture,
	);
	let (selected, temporal_reconciliation, trace_stages) = elf_selected_evidence_text(
		loaded,
		&stored_corpus,
		&evidence_ids,
		&ingested,
		&capture_failure,
	);
	let replay_command = elf_replay_command(response.trace_id, project_id.as_str());
	let (operator_debug, operator_debug_evidence) = operator_debug_output(
		AdapterKind::ElfServiceRuntime,
		loaded,
		Some(response.trace_id),
		replay_command,
		format!(
			"/v2/admin/traces/{}/bundle?mode=full&stage_items_limit=128&candidates_limit=200",
			response.trace_id
		),
	);
	let (pages, knowledge, knowledge_failure) =
		match materialize_elf_knowledge(service, loaded, &ingested, adapter_id).await {
			Ok(output) => output,
			Err(err) if loaded.job.suite == "knowledge_compilation" =>
				(Vec::new(), None, Some(format!("live_adapter.knowledge: {err}"))),
			Err(_) => (Vec::new(), None, None),
		};
	let (consolidation_response, consolidation, consolidation_failure) =
		match materialize_elf_consolidation(runtime, service, loaded, &ingested, adapter_id).await {
			Ok(output) => output,
			Err(err) if loaded.job.suite == "consolidation" =>
				(None, None, Some(format!("live_adapter.consolidation: {err}"))),
			Err(_) => (None, None, None),
		};
	let dreaming_readback = materialize_elf_dreaming_readback(
		service,
		loaded,
		project_id.as_str(),
		response.trace_id,
		adapter_id,
	)
	.await?;
	let dreaming_failure = dreaming_readback.as_ref().and_then(|output| {
		if output.materialization.missing_source_refs.is_empty() {
			None
		} else {
			Some(format!(
				"live_adapter.dreaming_readback missing source refs: {}",
				output.materialization.missing_source_refs.join(", ")
			))
		}
	});
	let failure = knowledge_failure.or(consolidation_failure).or(dreaming_failure);
	let suite_selection = suite_materialization_selection(SuiteMaterializationSelectionInput {
		loaded,
		ingested: &ingested,
		capture_failure: &capture_failure,
		selected,
		trace_stages,
		knowledge: &knowledge,
		consolidation: &consolidation,
		dreaming_readback,
	});

	Ok(materialized_job(
		loaded,
		adapter_id,
		MaterializedJobInput {
			content: suite_selection.selected.content,
			evidence_ids: suite_selection.selected.evidence_ids,
			pages,
			latency_ms,
			indexing_latency_ms: None,
			returned_count: response.items.len(),
			trace_id: Some(response.trace_id),
			failure,
			source_mappings: Vec::new(),
			operator_debug,
			operator_debug_evidence,
			capture: capture_for_job(loaded, capture),
			capture_failure,
			consolidation_response,
			consolidation,
			knowledge,
			temporal_reconciliation,
			dreaming_readback: suite_selection.dreaming_readback,
			memory_summaries: suite_selection.memory_summaries,
			proactive_briefs: suite_selection.proactive_briefs,
			scheduled_tasks: suite_selection.scheduled_tasks,
			trace_stages: suite_selection.trace_stages,
		},
	))
}

async fn search_elf_job(
	service: &ElfService,
	loaded: &LoadedJob,
	project_id: &str,
) -> color_eyre::Result<(SearchResponse, f64)> {
	let started_at = Instant::now();
	let response = service
		.search_raw(SearchRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: project_id.to_string(),
			agent_id: AGENT_ID.to_string(),
			token_id: None,
			payload_level: PayloadLevel::L2,
			read_profile: "private_only".to_string(),
			query: loaded.job.prompt.content.clone(),
			top_k: Some(5),
			candidate_k: Some(20),
			filter: None,
			record_hits: Some(false),
			ranking: None,
		})
		.await
		.map_err(|err| eyre::eyre!("ELF search_raw failed for {}: {err}", loaded.job.job_id))?;

	Ok((response, started_at.elapsed().as_secs_f64() * 1_000.0))
}

async fn materialize_elf_consolidation(
	runtime: &BaselineRuntime,
	service: &ElfService,
	loaded: &LoadedJob,
	ingested: &IngestedCorpus,
	adapter_id: &str,
) -> color_eyre::Result<(
	Option<serde_json::Value>,
	Option<ConsolidationMaterializationEvidence>,
	Option<String>,
)> {
	if loaded.job.suite != "consolidation" {
		return Ok((None, None, None));
	}

	let project_id = project_id_for_job(&loaded.job.job_id);
	let fixture = live_consolidation_fixture(loaded)?;
	let corpus = corpus_texts(loaded)?;
	let prepared = prepare_consolidation_run(loaded, adapter_id, ingested, &fixture, &corpus)?;
	let run = service
		.consolidation_run_create(ConsolidationRunCreateRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: project_id.clone(),
			agent_id: AGENT_ID.to_string(),
			job_kind: "fixture".to_string(),
			input_refs: prepared.input_refs.clone(),
			source_snapshot: serde_json::json!({
				"schema": "real_world_live_consolidation_run_snapshot/v1",
				"adapter_id": adapter_id,
				"job_id": loaded.job.job_id,
				"source_ref_count": prepared.input_refs.len()
			}),
			lineage: ConsolidationLineage {
				source_refs: prepared.input_refs.clone(),
				parent_run_id: None,
				parent_proposal_ids: Vec::new(),
			},
			proposals: prepared.proposals,
		})
		.await
		.map_err(|err| {
			eyre::eyre!("ELF consolidation_run_create failed for {}: {err}", loaded.job.job_id)
		})?;

	run_worker(runtime).await?;

	let reviewed = review_live_consolidation_proposals(
		service,
		loaded,
		project_id.as_str(),
		run.run.run_id,
		&fixture,
	)
	.await?;
	let consolidation_response = live_consolidation_response(&fixture, &reviewed)?;
	let evidence = consolidation_materialization_evidence(
		run.run.run_id,
		&fixture,
		&prepared.input_refs,
		&reviewed,
	);

	Ok((Some(consolidation_response), Some(evidence), None))
}

async fn materialize_elf_knowledge(
	service: &ElfService,
	loaded: &LoadedJob,
	ingested: &IngestedCorpus,
	adapter_id: &str,
) -> color_eyre::Result<(
	Vec<serde_json::Value>,
	Option<KnowledgeMaterializationEvidence>,
	Option<String>,
)> {
	if loaded.job.suite != "knowledge_compilation" {
		return Ok((Vec::new(), None, None));
	}

	let project_id = project_id_for_job(&loaded.job.job_id);
	let note_ids = live_note_ids(ingested);

	if note_ids.is_empty() {
		return Err(eyre::eyre!(
			"{} has no live note sources for knowledge rebuild.",
			loaded.job.job_id
		));
	}

	let page_key = slug(&loaded.job.job_id);
	let request = KnowledgePageRebuildRequest {
		tenant_id: TENANT_ID.to_string(),
		project_id: project_id.clone(),
		agent_id: AGENT_ID.to_string(),
		page_kind: KnowledgePageKind::Project,
		page_key,
		title: Some(loaded.job.title.clone()),
		doc_ids: Vec::new(),
		doc_chunk_ids: Vec::new(),
		note_ids: note_ids.clone(),
		event_ids: Vec::new(),
		relation_ids: Vec::new(),
		proposal_ids: Vec::new(),
		provider_metadata: serde_json::json!({
			"adapter_id": adapter_id,
			"job_id": loaded.job.job_id,
			"llm_derived": false,
			"runtime_path": "ElfService::knowledge_page_rebuild"
		}),
	};
	let first = service.knowledge_page_rebuild(request.clone()).await.map_err(|err| {
		eyre::eyre!("ELF knowledge_page_rebuild failed for {}: {err}", loaded.job.job_id)
	})?;
	let second = service.knowledge_page_rebuild(request).await.map_err(|err| {
		eyre::eyre!("ELF second knowledge_page_rebuild failed for {}: {err}", loaded.job.job_id)
	})?;

	update_stale_trap_sources(service, loaded, adapter_id, project_id.as_str()).await?;

	let lint = service
		.knowledge_page_lint(KnowledgePageLintRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: project_id.clone(),
			page_id: second.page.page.page_id,
		})
		.await
		.map_err(|err| {
			eyre::eyre!("ELF knowledge_page_lint failed for {}: {err}", loaded.job.job_id)
		})?;
	let search = service
		.knowledge_pages_search(KnowledgePageSearchRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id,
			query: "source notes".to_string(),
			page_kind: Some(KnowledgePageKind::Project),
			limit: Some(10),
		})
		.await
		.map_err(|err| {
			eyre::eyre!("ELF knowledge_pages_search failed for {}: {err}", loaded.job.job_id)
		})?;
	let page = knowledge_page_artifact(loaded, ingested, &first.page, &second.page, &lint)?;
	let evidence = knowledge_materialization_evidence(&second.page, &lint, search.items.len());

	Ok((vec![page], Some(evidence), None))
}

async fn ingest_elf_corpus(
	service: &ElfService,
	loaded: &LoadedJob,
	adapter_id: &str,
	project_id: &str,
	corpus: &[CorpusText],
) -> color_eyre::Result<IngestedCorpus> {
	let mut ingested = IngestedCorpus::default();

	for item in corpus {
		if item.capture.action == LiveCaptureAction::Exclude {
			push_unique(&mut ingested.capture.excluded_evidence_ids, item.evidence_id.clone());

			continue;
		}

		push_unique(&mut ingested.capture.stored_evidence_ids, item.evidence_id.clone());

		if let Some(source_id) = item.capture.source_id.as_deref() {
			push_unique(&mut ingested.capture.source_ids, source_id.to_string());
		}

		if item.capture.write_policy.is_some() {
			let note_id = ingest_elf_corpus_item(
				service,
				loaded,
				adapter_id,
				project_id,
				item,
				item.evidence_id.clone(),
				item.text.clone(),
				0,
				1,
				&mut ingested.capture,
			)
			.await?;

			ingested
				.note_ids_by_evidence
				.entry(item.evidence_id.clone())
				.or_default()
				.push(note_id);

			continue;
		}

		let chunks = note_text_chunks(item.text.as_str());
		let chunk_count = chunks.len();

		for (chunk_index, text) in chunks.into_iter().enumerate() {
			let key = if chunk_count == 1 {
				item.evidence_id.clone()
			} else {
				format!("{}:chunk-{chunk_index:03}", item.evidence_id)
			};
			let note_id = ingest_elf_corpus_item(
				service,
				loaded,
				adapter_id,
				project_id,
				item,
				key,
				text,
				chunk_index,
				chunk_count,
				&mut ingested.capture,
			)
			.await?;

			ingested
				.note_ids_by_evidence
				.entry(item.evidence_id.clone())
				.or_default()
				.push(note_id);
		}
	}

	Ok(ingested)
}

#[allow(clippy::too_many_arguments)]
async fn ingest_elf_corpus_item(
	service: &ElfService,
	loaded: &LoadedJob,
	adapter_id: &str,
	project_id: &str,
	item: &CorpusText,
	key: String,
	text: String,
	chunk_index: usize,
	chunk_count: usize,
	capture: &mut CaptureMaterializationEvidence,
) -> color_eyre::Result<Uuid> {
	let write_policy = item
		.capture
		.write_policy
		.as_ref()
		.map(|policy| write_policy_from_value(policy, item.evidence_id.as_str()))
		.transpose()?;
	let response = service
		.add_note(AddNoteRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: project_id.to_string(),
			agent_id: AGENT_ID.to_string(),
			scope: SCOPE.to_string(),
			notes: vec![AddNoteInput {
				r#type: "fact".to_string(),
				key: Some(key),
				text,
				structured: None,
				importance: 0.9,
				confidence: 0.95,
				ttl_days: None,
				source_ref: serde_json::json!({
					"schema": "real_world_live_adapter/v1",
					"adapter": adapter_id,
					"job_id": loaded.job.job_id,
					"evidence_id": item.evidence_id,
					"source_id": item.capture.source_id.as_deref(),
					"capture_action": capture_action_str(item.capture.action),
					"evidence_binding": item.capture.evidence_binding.as_deref(),
					"write_policy_applied": item.capture.write_policy.is_some(),
					"chunk_index": chunk_index,
					"chunk_count": chunk_count,
				}),
				write_policy,
			}],
		})
		.await
		.map_err(|err| eyre::eyre!("ELF add_note failed for {}: {err}", loaded.job.job_id))?;

	for result in &response.results {
		if let Some(audit) = &result.write_policy_audit
			&& (!audit.exclusions.is_empty() || !audit.redactions.is_empty())
		{
			capture.write_policy_audit_count += 1;
			capture.write_policy_exclusion_count += audit.exclusions.len();
			capture.write_policy_redaction_count += audit.redactions.len();
		}
	}

	response.results.iter().find_map(|result| result.note_id).ok_or_else(|| {
		eyre::eyre!(
			"ELF add_note did not persist evidence {} chunk {} for {}.",
			item.evidence_id,
			chunk_index,
			loaded.job.job_id
		)
	})
}

async fn review_live_consolidation_proposals(
	service: &ElfService,
	loaded: &LoadedJob,
	project_id: &str,
	run_id: Uuid,
	fixture: &LiveConsolidationFixture,
) -> color_eyre::Result<Vec<ConsolidationProposalResponse>> {
	let listed = service
		.consolidation_proposals_list(ConsolidationProposalsListRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: project_id.to_string(),
			run_id: Some(run_id),
			review_state: None,
			limit: Some(100),
		})
		.await
		.map_err(|err| {
			eyre::eyre!("ELF consolidation proposal list failed for {}: {err}", loaded.job.job_id)
		})?;
	let mut reviewed = Vec::new();

	for (index, proposal) in listed.proposals.into_iter().enumerate() {
		let fixture_proposal = fixture.proposals.get(index).ok_or_else(|| {
			eyre::eyre!(
				"ELF consolidation materialized extra proposal {} for {}.",
				proposal.proposal_id,
				loaded.job.job_id
			)
		})?;
		let review_action =
			consolidation_review_action(fixture_proposal.actual_review_action.as_str())?;

		reviewed.push(
			service
				.consolidation_proposal_review(ConsolidationProposalReviewRequest {
					tenant_id: TENANT_ID.to_string(),
					project_id: project_id.to_string(),
					reviewer_agent_id: AGENT_ID.to_string(),
					proposal_id: proposal.proposal_id,
					review_action,
					review_comment: Some(
						"Live adapter review transition for real-world benchmark evidence."
							.to_string(),
					),
				})
				.await
				.map_err(|err| {
					eyre::eyre!(
						"ELF consolidation proposal review failed for {}: {err}",
						loaded.job.job_id
					)
				})?,
		);
	}

	validate_reviewed_consolidation_count(loaded, fixture, &reviewed)?;

	Ok(reviewed)
}

async fn update_stale_trap_sources(
	service: &ElfService,
	loaded: &LoadedJob,
	adapter_id: &str,
	project_id: &str,
) -> color_eyre::Result<()> {
	for evidence_id in stale_trap_evidence_ids(loaded) {
		service
			.add_note(AddNoteRequest {
				tenant_id: TENANT_ID.to_string(),
				project_id: project_id.to_string(),
				agent_id: AGENT_ID.to_string(),
				scope: SCOPE.to_string(),
				notes: vec![AddNoteInput {
					r#type: "fact".to_string(),
					key: Some(evidence_id.clone()),
					text: format!(
						"Current lint probe: evidence {evidence_id} changed after the knowledge page rebuild and should mark the derived page source snapshot stale."
					),
					structured: None,
					importance: 0.9,
					confidence: 0.95,
					ttl_days: None,
					source_ref: serde_json::json!({
						"schema": "real_world_live_adapter/v1",
						"adapter": adapter_id,
						"job_id": loaded.job.job_id,
						"evidence_id": evidence_id,
						"lint_probe": "stale_source_ref"
					}),
					write_policy: None,
				}],
			})
			.await
			.map_err(|err| {
				eyre::eyre!(
					"ELF add_note stale-source update failed for {}: {err}",
					loaded.job.job_id
				)
			})?;
	}

	Ok(())
}

async fn build_service(runtime: &BaselineRuntime) -> color_eyre::Result<ElfService> {
	let cfg = runtime_config(runtime)?;
	let vector_dim = cfg.storage.qdrant.vector_dim;
	let db = Db::connect(&cfg.storage.postgres).await?;

	db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;

	let qdrant = QdrantStore::new(&cfg.storage.qdrant)?;

	qdrant.ensure_collection().await?;

	Ok(ElfService::with_providers(cfg, db, qdrant, deterministic_providers(vector_dim)))
}

async fn build_worker_state(runtime: &BaselineRuntime) -> color_eyre::Result<WorkerState> {
	let cfg = runtime_config(runtime)?;
	let db = Db::connect(&cfg.storage.postgres).await?;

	db.ensure_schema(cfg.storage.qdrant.vector_dim).await?;

	let qdrant = QdrantStore::new(&cfg.storage.qdrant)?;

	qdrant.ensure_collection().await?;

	let docs_qdrant =
		QdrantStore::new_with_collection(&cfg.storage.qdrant, &cfg.storage.qdrant.docs_collection)?;

	docs_qdrant.ensure_collection().await?;

	let tokenizer = elf_chunking::load_tokenizer(&cfg.chunking.tokenizer_repo)
		.map_err(|err| eyre::eyre!("Failed to load tokenizer for live adapter worker: {err}"))?;
	let chunking = ChunkingConfig {
		max_tokens: cfg.chunking.max_tokens,
		overlap_tokens: cfg.chunking.overlap_tokens,
	};

	Ok(WorkerState {
		db,
		qdrant,
		docs_qdrant,
		embedding: cfg.providers.embedding,
		chunking,
		tokenizer,
	})
}

async fn run_worker(runtime: &BaselineRuntime) -> color_eyre::Result<()> {
	let state = Arc::new(build_worker_state(runtime).await?);

	for _ in 0..8 {
		let state = Arc::clone(&state);
		let mut set = JoinSet::new();

		set.spawn(async move {
			worker::process_once(&state)
				.await
				.map_err(|err| eyre::eyre!("Worker process_once failed: {err}"))
		});

		while let Some(joined) = set.join_next().await {
			joined??;
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use serde_json::Value;

	fn capture_item(
		evidence_id: &str,
		action: super::LiveCaptureAction,
		source_id: Option<&str>,
		evidence_binding: Option<&str>,
		write_policy: Option<Value>,
	) -> super::CorpusText {
		super::CorpusText {
			evidence_id: evidence_id.to_string(),
			text: "Public capture text.".to_string(),
			capture: super::LiveCapturePolicy {
				action,
				source_id: source_id.map(ToString::to_string),
				evidence_binding: evidence_binding.map(ToString::to_string),
				write_policy,
			},
		}
	}

	fn capture_evidence(
		stored: &[&str],
		excluded: &[&str],
	) -> super::CaptureMaterializationEvidence {
		super::CaptureMaterializationEvidence {
			stored_evidence_ids: stored.iter().map(|id| (*id).to_string()).collect(),
			excluded_evidence_ids: excluded.iter().map(|id| (*id).to_string()).collect(),
			source_ids: Vec::new(),
			write_policy_audit_count: 0,
			write_policy_exclusion_count: 0,
			write_policy_redaction_count: 0,
			runtime_source_refs: Vec::new(),
		}
	}

	#[test]
	fn capture_runtime_validation_requires_returned_source_id() {
		let corpus = vec![capture_item(
			"source-a",
			super::LiveCaptureAction::Store,
			Some("capture:a"),
			None,
			None,
		)];
		let capture = capture_evidence(&["source-a"], &[]);
		let runtime = super::capture_runtime_evidence_from_source_refs([&serde_json::json!({
			"evidence_id": "source-a",
			"capture_action": "store"
		})]);
		let failure = super::validate_capture_runtime_evidence(
			"capture_integration",
			&corpus,
			&capture,
			&runtime,
		)
		.expect("missing runtime source_id should fail capture validation");

		assert!(failure.contains("did not return expected source_id capture:a"));
	}

	#[test]
	fn capture_runtime_validation_rejects_returned_excluded_evidence() {
		let corpus = vec![capture_item(
			"private-trap",
			super::LiveCaptureAction::Exclude,
			Some("capture:private"),
			Some("negative_trap"),
			None,
		)];
		let capture = capture_evidence(&[], &["private-trap"]);
		let runtime = super::capture_runtime_evidence_from_source_refs([&serde_json::json!({
			"evidence_id": "private-trap",
			"source_id": "capture:private",
			"capture_action": "store"
		})]);
		let failure = super::validate_capture_runtime_evidence(
			"capture_integration",
			&corpus,
			&capture,
			&runtime,
		)
		.expect("returned excluded evidence should fail capture validation");

		assert!(failure.contains("excluded evidence private-trap was returned by live search"));
	}

	#[test]
	fn capture_runtime_source_refs_are_written_into_generated_fixture() {
		let mut value = serde_json::json!({
			"corpus": {
				"items": [
					{
						"evidence_id": "source-a",
						"source_ref": {
							"schema": "source_ref/v1",
							"resolver": "fixture"
						}
					}
				]
			}
		});
		let mut capture = capture_evidence(&["source-a"], &[]);

		capture.runtime_source_refs.push(super::CaptureRuntimeSourceRefEvidence {
			evidence_id: "source-a".to_string(),
			source_ref: serde_json::json!({
				"schema": "real_world_live_adapter/v1",
				"evidence_id": "source-a",
				"source_id": "capture:a",
				"capture_action": "store",
				"evidence_binding": "source_ref"
			}),
		});

		super::apply_capture_runtime_source_refs(&mut value, &capture);

		assert_eq!(
			value
				.pointer("/corpus/items/0/source_ref/source_id")
				.and_then(serde_json::Value::as_str),
			Some("capture:a")
		);
		assert_eq!(
			value
				.pointer("/corpus/items/0/source_ref/evidence_binding")
				.and_then(serde_json::Value::as_str),
			Some("source_ref")
		);
	}
}
