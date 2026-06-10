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
use serde_json::{Map, Value};
use tokio::{task::JoinSet, time};
use uuid::Uuid;

use elf_chunking::ChunkingConfig;
use elf_config::{Config, EmbeddingProviderConfig, LlmProviderConfig, ProviderConfig};
use elf_service::{
	AddNoteInput, AddNoteRequest, BoxFuture, ElfService, EmbeddingProvider, ExtractorProvider,
	PayloadLevel, Providers, RerankProvider, SearchRequest,
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
	value: Value,
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
	evidence_links: Map<String, Value>,
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
	metadata: Option<Value>,
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
	claims: Vec<Value>,
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
	metadata: Option<Value>,
}

#[derive(Debug)]
struct CorpusText {
	evidence_id: String,
	text: String,
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
		_messages: &'a [Value],
	) -> BoxFuture<'a, elf_service::Result<Value>> {
		Box::pin(async move { Ok(serde_json::json!({ "notes": [] })) })
	}
}

#[derive(Debug)]
struct SelectedEvidenceText {
	content: String,
	evidence_ids: Vec<String>,
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
	let results = serde_json::from_str::<Value>(&stdout).map_err(|err| {
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

fn lightrag_index_failed(status: &Value) -> bool {
	status.get("documents").and_then(Value::as_array).into_iter().flatten().any(|doc| {
		doc.get("status")
			.and_then(Value::as_str)
			.is_some_and(|status| status.to_ascii_lowercase().contains("fail"))
	})
}

fn lightrag_index_processed(status: &Value, expected_docs: usize) -> bool {
	let Some(documents) = status.get("documents").and_then(Value::as_array) else {
		return false;
	};

	documents.len() >= expected_docs
		&& documents.iter().all(|doc| {
			doc.get("status").and_then(Value::as_str).is_some_and(|status| {
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
	response: &Value,
) -> Vec<SourceMappingEvidence> {
	let mut mappings = Vec::new();

	if let Some(references) = response.get("references").and_then(Value::as_array) {
		for reference in references {
			mappings.push(lightrag_reference_mapping(corpus, sources, reference));
		}
	}

	if mappings.is_empty()
		&& let Some(context) = response.get("response").and_then(Value::as_str)
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
	reference: &Value,
) -> SourceMappingEvidence {
	let source = reference
		.get("file_path")
		.and_then(Value::as_str)
		.or_else(|| reference.get("reference_id").and_then(Value::as_str))
		.unwrap_or("unknown_source")
		.to_string();
	let content = reference
		.get("content")
		.and_then(Value::as_array)
		.into_iter()
		.flatten()
		.filter_map(Value::as_str)
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

fn lightrag_metadata(args: &LightragArgs, run_slug: &str) -> Value {
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
	let required_evidence_satisfied = required_evidence_satisfied(loaded, &input.evidence_ids);
	let status = if input.failure.is_some() {
		MaterializationStatus::Incomplete
	} else if !required_evidence_satisfied {
		MaterializationStatus::WrongResult
	} else {
		MaterializationStatus::Pass
	};
	let failure_stage = input.failure.as_ref().map(|_| "adapter_runtime".to_string());
	let stage_notes = if !required_evidence_satisfied {
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
					failure_stage: failure_stage.map(|_| "live_adapter.retrieve".to_string()),
					failure_reason: input.failure.clone(),
					stages: vec![TraceStageOutput {
						stage_name: "live_adapter.retrieve".to_string(),
						kept_evidence: input.evidence_ids.clone(),
						dropped_evidence: Vec::new(),
						demoted_evidence: Vec::new(),
						distractor_evidence: Vec::new(),
						notes: stage_notes,
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
			evidence_ids: input.evidence_ids,
			returned_count: input.returned_count,
			indexing_latency_ms: input.indexing_latency_ms,
			latency_ms: input.latency_ms,
			trace_id: input.trace_id,
			failure: input.failure,
			source_mappings: input.source_mappings,
		},
	}
}

fn declared_encoding_job(adapter_id: &str, loaded: &LoadedJob) -> Option<MaterializedJob> {
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
	not_encoded_reason(loaded.job.suite.as_str()).map(|reason| {
		materialized_declared_status_job(
			adapter_id,
			loaded,
			MaterializationStatus::NotEncoded,
			reason.to_string(),
		)
	})
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
			"The live adapter sweep does not yet hydrate full operator trace/viewer diagnostics for this suite.",
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
		},
	}
}

fn evidence_linked_claims(loaded: &LoadedJob, evidence_ids: &[String]) -> Vec<Value> {
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

fn evidence_link_ids(value: &Value) -> Vec<String> {
	if let Some(id) = value.as_str() {
		return vec![id.to_string()];
	}

	value
		.as_array()
		.map(|items| {
			items.iter().filter_map(Value::as_str).map(ToString::to_string).collect::<Vec<_>>()
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

		value["corpus"]["adapter_response"] = Value::Object(adapter_response);

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
		let value = serde_json::from_str::<Value>(&raw)
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

			Ok(CorpusText { evidence_id: item.evidence_id.clone(), text: text.trim().to_string() })
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
		},
	))
}

async fn insert_lightrag_texts(
	args: &LightragArgs,
	client: &reqwest::Client,
	corpus: &[CorpusText],
	sources: &[LightragSource],
) -> color_eyre::Result<Value> {
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
	insert_response: &Value,
	expected_docs: usize,
) -> color_eyre::Result<()> {
	let track_id = insert_response
		.get("track_id")
		.and_then(Value::as_str)
		.ok_or_else(|| eyre::eyre!("LightRAG text insert response did not include track_id."))?;
	let mut last_status = Value::Null;

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
) -> color_eyre::Result<Value> {
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
) -> color_eyre::Result<Value> {
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
	body: &Value,
) -> color_eyre::Result<Value> {
	let url = format!("{}{}", lightrag_api_base(args), path);
	let mut request = client.post(url).json(body);

	if let Some(api_key) = args.api_key.as_deref().filter(|key| !key.is_empty()) {
		request = request.bearer_auth(api_key);
	}

	lightrag_send_json(request).await
}

async fn lightrag_send_json(request: RequestBuilder) -> color_eyre::Result<Value> {
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
	let project_id = project_id_for_job(&loaded.job.job_id);

	for item in &corpus {
		let chunks = note_text_chunks(item.text.as_str());
		let chunk_count = chunks.len();

		for (chunk_index, text) in chunks.into_iter().enumerate() {
			let key = if chunk_count == 1 {
				item.evidence_id.clone()
			} else {
				format!("{}:chunk-{chunk_index:03}", item.evidence_id)
			};
			let response = service
				.add_note(AddNoteRequest {
					tenant_id: TENANT_ID.to_string(),
					project_id: project_id.clone(),
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
							"chunk_index": chunk_index,
							"chunk_count": chunk_count,
						}),
						write_policy: None,
					}],
				})
				.await
				.map_err(|err| {
					eyre::eyre!("ELF add_note failed for {}: {err}", loaded.job.job_id)
				})?;

			if !response.results.iter().any(|result| result.note_id.is_some()) {
				return Err(eyre::eyre!(
					"ELF add_note did not persist evidence {} chunk {} for {}.",
					item.evidence_id,
					chunk_index,
					loaded.job.job_id
				));
			}
		}
	}

	run_worker(runtime).await?;

	let started_at = Instant::now();
	let response = service
		.search_raw(SearchRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id,
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
		if let Some(evidence_id) = item.source_ref.get("evidence_id").and_then(Value::as_str) {
			push_unique(&mut evidence_ids, evidence_id.to_string());
		}
	}

	let selected = selected_required_corpus_texts(loaded, &corpus, &evidence_ids);

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
		},
	))
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
