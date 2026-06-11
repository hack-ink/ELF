#![allow(clippy::single_component_path_imports, unused_crate_dependencies)]

//! Live adapter materializer for the real-world job benchmark.

use std::{
	collections::BTreeSet,
	env,
	fs::{self, OpenOptions},
	io::Write as _,
	path::{Path, PathBuf},
	process::{Command, Stdio},
	sync::Arc,
	time::{Duration, Instant},
};

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
use elf_domain::writegate::{self, WritePolicy};
use elf_service::{
	AddNoteInput, AddNoteRequest, BoxFuture, ElfService, EmbeddingProvider, ExtractorProvider,
	PayloadLevel, Providers, RerankProvider, SearchItem, SearchRequest,
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
}

#[derive(Debug, Serialize)]
struct AnswerOutput {
	content: String,
	evidence_ids: Vec<String>,
	claims: Vec<serde_json::Value>,
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

#[derive(Debug, Serialize)]
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

	Ok(materialized_job(
		loaded,
		&args.adapter_id,
		MaterializedJobInput {
			content: selected.content,
			evidence_ids: selected.evidence_ids,
			latency_ms,
			indexing_latency_ms: None,
			returned_count: entries.len(),
			trace_id: None,
			failure: None,
			source_mappings: Vec::new(),
			operator_debug,
			operator_debug_evidence,
			capture: None,
			capture_failure: None,
		},
	))
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

	MaterializedJob {
		response: AdapterResponseOutput {
			adapter_id: adapter_id.to_string(),
			answer: AnswerOutput {
				content: input.content,
				evidence_ids: input.evidence_ids.clone(),
				claims: evidence_linked_claims(loaded, &input.evidence_ids),
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
					stages: vec![TraceStageOutput {
						stage_name: failure_stage
							.unwrap_or_else(|| "live_adapter.retrieve".to_string()),
						kept_evidence: input.evidence_ids.clone(),
						dropped_evidence: Vec::new(),
						demoted_evidence: Vec::new(),
						distractor_evidence: Vec::new(),
						notes: stage_notes,
					}],
				},
			},
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
		},
	}
}

fn declared_encoding_job(adapter_id: &str, loaded: &LoadedJob) -> Option<MaterializedJob> {
	if is_operator_debug_live_adapter(adapter_id, loaded.job.suite.as_str()) {
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
	if is_elf_capture_live_adapter(adapter_id, loaded.job.suite.as_str()) {
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
		&& matches!(adapter_id, "elf_operator_debug_live" | "qmd_operator_debug_live")
}

fn is_elf_capture_live_adapter(adapter_id: &str, suite: &str) -> bool {
	suite == "capture_integration"
		&& matches!(adapter_id, "elf_live_real_world" | "elf_capture_write_policy_live")
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
	let capture =
		ingest_elf_corpus(service, loaded, adapter_id, project_id.as_str(), &corpus).await?;

	run_worker(runtime).await?;

	let started_at = Instant::now();
	let response = service
		.search_raw(SearchRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: project_id.clone(),
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
	let latency_ms = started_at.elapsed().as_secs_f64() * 1_000.0;
	let mut evidence_ids = Vec::new();

	for item in &response.items {
		if let Some(evidence_id) =
			item.source_ref.get("evidence_id").and_then(serde_json::Value::as_str)
		{
			push_unique(&mut evidence_ids, evidence_id.to_string());
		}
	}

	let runtime_capture = capture_runtime_evidence_from_search_items(&response.items);
	let capture = capture_with_runtime_source_refs(capture, &runtime_capture);
	let capture_failure = validate_capture_runtime_evidence(
		loaded.job.suite.as_str(),
		&corpus,
		&capture,
		&runtime_capture,
	);
	let selected = if let Some(failure) = &capture_failure {
		SelectedEvidenceText { content: failure.clone(), evidence_ids: Vec::new() }
	} else {
		selected_required_corpus_texts(loaded, &stored_corpus, &evidence_ids)
	};
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

	Ok(materialized_job(
		loaded,
		adapter_id,
		MaterializedJobInput {
			content: selected.content,
			evidence_ids: selected.evidence_ids,
			latency_ms,
			indexing_latency_ms: None,
			returned_count: response.items.len(),
			trace_id: Some(response.trace_id),
			failure: None,
			source_mappings: Vec::new(),
			operator_debug,
			operator_debug_evidence,
			capture: capture_for_job(loaded, capture),
			capture_failure,
		},
	))
}

async fn ingest_elf_corpus(
	service: &ElfService,
	loaded: &LoadedJob,
	adapter_id: &str,
	project_id: &str,
	corpus: &[CorpusText],
) -> color_eyre::Result<CaptureMaterializationEvidence> {
	let mut capture = CaptureMaterializationEvidence::default();

	for item in corpus {
		if item.capture.action == LiveCaptureAction::Exclude {
			push_unique(&mut capture.excluded_evidence_ids, item.evidence_id.clone());

			continue;
		}

		push_unique(&mut capture.stored_evidence_ids, item.evidence_id.clone());

		if let Some(source_id) = item.capture.source_id.as_deref() {
			push_unique(&mut capture.source_ids, source_id.to_string());
		}

		if item.capture.write_policy.is_some() {
			ingest_elf_corpus_item(
				service,
				loaded,
				adapter_id,
				project_id,
				item,
				item.evidence_id.clone(),
				item.text.clone(),
				0,
				1,
				&mut capture,
			)
			.await?;

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

			ingest_elf_corpus_item(
				service,
				loaded,
				adapter_id,
				project_id,
				item,
				key,
				text,
				chunk_index,
				chunk_count,
				&mut capture,
			)
			.await?;
		}
	}

	Ok(capture)
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
) -> color_eyre::Result<()> {
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

	if !response.results.iter().any(|result| result.note_id.is_some()) {
		return Err(eyre::eyre!(
			"ELF add_note did not persist evidence {} chunk {} for {}.",
			item.evidence_id,
			chunk_index,
			loaded.job.job_id
		));
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
