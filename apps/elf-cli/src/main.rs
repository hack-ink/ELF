//! Local ELF CLI wrappers for production memory workflows.

use std::{
	collections::BTreeMap,
	io::{self, Write as _},
	path::{Path, PathBuf},
	process::Command,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use color_eyre::{Result, eyre};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode, header::HeaderMap};
use serde_json::{self, Value};

const DEFAULT_API_URL: &str = "http://127.0.0.1:51892";
const DEFAULT_ADMIN_URL: &str = "http://127.0.0.1:51891";
const DEFAULT_TENANT_ID: &str = "local-tenant";
const DEFAULT_PROJECT_ID: &str = "local-project";
const DEFAULT_AGENT_ID: &str = "local-agent";
const DEFAULT_READ_PROFILE: &str = "private_only";

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
	about = "Local ELF workflow wrappers over the HTTP API and repo benchmark tasks."
)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Debug, Args)]
struct PublicEndpointArgs {
	/// Public ELF API base URL.
	#[arg(long, env = "ELF_API_URL", default_value = DEFAULT_API_URL)]
	api_url: String,
	/// Optional bearer token for static-key auth.
	#[arg(long, env = "ELF_USER_TOKEN")]
	token: Option<String>,
}

#[derive(Debug, Args)]
struct AdminEndpointArgs {
	/// Admin ELF API base URL.
	#[arg(long, env = "ELF_ADMIN_URL", default_value = DEFAULT_ADMIN_URL)]
	admin_url: String,
	/// Optional admin bearer token for static-key auth.
	#[arg(long, env = "ELF_ADMIN_TOKEN")]
	admin_token: Option<String>,
}

#[derive(Clone, Debug, Args)]
struct ContextArgs {
	/// Tenant id sent in X-ELF-Tenant-Id.
	#[arg(long, env = "ELF_TENANT_ID", default_value = DEFAULT_TENANT_ID)]
	tenant_id: String,
	/// Project id sent in X-ELF-Project-Id.
	#[arg(long, env = "ELF_PROJECT_ID", default_value = DEFAULT_PROJECT_ID)]
	project_id: String,
	/// Agent id sent in X-ELF-Agent-Id.
	#[arg(long, env = "ELF_AGENT_ID", default_value = DEFAULT_AGENT_ID)]
	agent_id: String,
}

#[derive(Clone, Debug, Args)]
struct ReadContextArgs {
	#[command(flatten)]
	context: ContextArgs,
	/// Read profile sent in X-ELF-Read-Profile.
	#[arg(long, env = "ELF_READ_PROFILE", default_value = DEFAULT_READ_PROFILE)]
	read_profile: String,
}

#[derive(Debug, Args)]
struct OutputArgs {
	/// Pretty-print the JSON output.
	#[arg(long)]
	pretty: bool,
}

#[derive(Debug, Args)]
struct AddNoteArgs {
	#[command(flatten)]
	endpoint: PublicEndpointArgs,
	#[command(flatten)]
	context: ContextArgs,
	#[command(flatten)]
	output: OutputArgs,
	/// Scope applied to the note.
	#[arg(long, default_value = "agent_private")]
	scope: String,
	/// Memory note type.
	#[arg(long = "type", default_value = "fact")]
	note_type: String,
	/// Optional note key used by the update resolver.
	#[arg(long)]
	key: Option<String>,
	/// English note text.
	#[arg(long)]
	text: String,
	/// Ranking importance value.
	#[arg(long, default_value_t = 0.7)]
	importance: f32,
	/// Ranking confidence value.
	#[arg(long, default_value_t = 0.9)]
	confidence: f32,
	/// Optional TTL override in days.
	#[arg(long)]
	ttl_days: Option<i64>,
	/// Operator-visible source id copied into source_ref.ref.source_id.
	#[arg(long)]
	source_id: Option<String>,
	/// Full JSON object source_ref override.
	#[arg(long)]
	source_ref_json: Option<String>,
}

#[derive(Debug, Args)]
struct SearchArgs {
	#[command(flatten)]
	endpoint: PublicEndpointArgs,
	#[command(flatten)]
	read_context: ReadContextArgs,
	#[command(flatten)]
	output: OutputArgs,
	/// English query string.
	#[arg(long)]
	query: String,
	/// Search mode to request from the service.
	#[arg(long, value_enum, default_value_t = SearchMode::QuickFind)]
	mode: SearchMode,
	/// Number of final items to return.
	#[arg(long)]
	top_k: Option<u32>,
	/// Candidate breadth before ranking.
	#[arg(long)]
	candidate_k: Option<u32>,
	/// Payload level requested from the service.
	#[arg(long, value_enum, default_value_t = PayloadLevel::L0)]
	payload_level: PayloadLevel,
	/// Optional search filter JSON object.
	#[arg(long)]
	filter_json: Option<String>,
}

#[derive(Debug, Args)]
struct StatusArgs {
	#[command(flatten)]
	endpoint: PublicEndpointArgs,
	#[command(flatten)]
	output: OutputArgs,
}

#[derive(Debug, Args)]
struct BackfillArgs {
	#[command(flatten)]
	output: OutputArgs,
	/// Backfill corpus document count override.
	#[arg(long)]
	docs: Option<u32>,
	/// Worker concurrency override for the backfill runner.
	#[arg(long)]
	worker_concurrency: Option<u32>,
	/// Use the checked-in 10k operator profile task.
	#[arg(long)]
	ten_k: bool,
	/// Use the guarded 100k operator profile task.
	#[arg(long, conflicts_with = "ten_k")]
	hundred_k: bool,
	/// Set the required expensive-run guard for the 100k task.
	#[arg(long)]
	enable_expensive: bool,
	/// Print the resolved task and environment without running it.
	#[arg(long)]
	dry_run: bool,
}

#[derive(Debug, Args)]
struct BenchmarkArgs {
	#[command(subcommand)]
	command: BenchmarkCommand,
}

#[derive(Debug, Args)]
struct BenchmarkRunArgs {
	#[command(flatten)]
	output: OutputArgs,
	/// Benchmark task wrapper to run.
	#[arg(long, value_enum, default_value_t = BenchmarkRunKind::Live)]
	kind: BenchmarkRunKind,
	/// Project filter passed to ELF_BASELINE_PROJECTS.
	#[arg(long)]
	projects: Option<String>,
	/// Corpus profile passed to ELF_BASELINE_PROFILE.
	#[arg(long)]
	profile: Option<String>,
	/// Private production corpus manifest path.
	#[arg(long)]
	production_corpus_manifest: Option<PathBuf>,
	/// Markdown addendum path for production-private-addendum.
	#[arg(long)]
	private_addendum: Option<PathBuf>,
	/// Soak duration override in seconds.
	#[arg(long)]
	soak_seconds: Option<u32>,
	/// Print the resolved task and environment without running it.
	#[arg(long)]
	dry_run: bool,
}

#[derive(Debug, Args)]
struct BenchmarkReportArgs {
	#[command(flatten)]
	output: OutputArgs,
	/// Source live-baseline report JSON path.
	#[arg(long)]
	report: Option<PathBuf>,
	/// Markdown output path.
	#[arg(long)]
	out: Option<PathBuf>,
	/// Print the resolved task and environment without running it.
	#[arg(long)]
	dry_run: bool,
}

#[derive(Debug, Args)]
struct DiagnosticsArgs {
	#[command(subcommand)]
	command: DiagnosticsCommand,
}

#[derive(Debug, Args)]
struct AdminPostArgs {
	#[command(flatten)]
	endpoint: AdminEndpointArgs,
	#[command(flatten)]
	context: ContextArgs,
	#[command(flatten)]
	output: OutputArgs,
}

#[derive(Debug, Args)]
struct AdminSearchArgs {
	#[command(flatten)]
	endpoint: AdminEndpointArgs,
	#[command(flatten)]
	read_context: ReadContextArgs,
	#[command(flatten)]
	output: OutputArgs,
	/// English query string.
	#[arg(long)]
	query: String,
	/// Search mode to request from the service.
	#[arg(long, value_enum, default_value_t = SearchMode::QuickFind)]
	mode: SearchMode,
	/// Number of final items to return.
	#[arg(long)]
	top_k: Option<u32>,
	/// Candidate breadth before ranking.
	#[arg(long)]
	candidate_k: Option<u32>,
	/// Payload level requested from the service.
	#[arg(long, value_enum, default_value_t = PayloadLevel::L2)]
	payload_level: PayloadLevel,
	/// Optional search filter JSON object.
	#[arg(long)]
	filter_json: Option<String>,
}

#[derive(Debug, Args)]
struct RecentTracesArgs {
	#[command(flatten)]
	endpoint: AdminEndpointArgs,
	#[command(flatten)]
	context: ContextArgs,
	#[command(flatten)]
	output: OutputArgs,
	/// Maximum trace headers to return.
	#[arg(long)]
	limit: Option<u32>,
}

#[derive(Debug, Args)]
struct TraceBundleArgs {
	#[command(flatten)]
	endpoint: AdminEndpointArgs,
	#[command(flatten)]
	context: ContextArgs,
	#[command(flatten)]
	output: OutputArgs,
	/// Trace id to load.
	#[arg(long)]
	trace_id: String,
	/// Bundle mode: bounded or full.
	#[arg(long, default_value = "bounded")]
	mode: String,
	/// Optional per-stage item cap.
	#[arg(long)]
	stage_items_limit: Option<u32>,
	/// Optional replay candidate cap.
	#[arg(long)]
	candidates_limit: Option<u32>,
}

#[derive(Debug, Args)]
struct NoteProvenanceArgs {
	#[command(flatten)]
	endpoint: AdminEndpointArgs,
	#[command(flatten)]
	context: ContextArgs,
	#[command(flatten)]
	output: OutputArgs,
	/// Note id to inspect.
	#[arg(long)]
	note_id: String,
}

struct JsonRequest<'a> {
	method: Method,
	base_url: &'a str,
	path: &'a str,
	token: Option<&'a str>,
	context: Option<&'a ContextArgs>,
	read_profile: Option<&'a str>,
	body: Option<&'a Value>,
}

#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab")]
enum Commands {
	/// Add one deterministic note through POST /v2/notes/ingest.
	AddNote(AddNoteArgs),
	/// Create a search session through POST /v2/searches.
	Search(SearchArgs),
	/// Check local API process health.
	Status(StatusArgs),
	/// Run the checked-in resumable backfill benchmark workflow.
	Backfill(BackfillArgs),
	/// Run or render checked-in live baseline benchmark reports.
	Benchmark(BenchmarkArgs),
	/// Read production diagnostics through admin HTTP endpoints.
	Diagnostics(DiagnosticsArgs),
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "snake_case")]
enum SearchMode {
	QuickFind,
	PlannedSearch,
}
impl SearchMode {
	fn as_str(self) -> &'static str {
		match self {
			Self::QuickFind => "quick_find",
			Self::PlannedSearch => "planned_search",
		}
	}
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "lower")]
enum PayloadLevel {
	L0,
	L1,
	L2,
}
impl PayloadLevel {
	fn as_str(self) -> &'static str {
		match self {
			Self::L0 => "l0",
			Self::L1 => "l1",
			Self::L2 => "l2",
		}
	}
}

#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab")]
enum BenchmarkCommand {
	/// Run one checked-in Docker baseline task.
	Run(BenchmarkRunArgs),
	/// Render Markdown from a live-baseline JSON report.
	Report(BenchmarkReportArgs),
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab")]
enum BenchmarkRunKind {
	Live,
	ProductionSynthetic,
	ProductionPrivate,
	ProductionPrivateAddendum,
	Soak,
}
impl BenchmarkRunKind {
	fn task_name(self) -> &'static str {
		match self {
			Self::Live => "baseline-live-docker",
			Self::ProductionSynthetic => "baseline-production-synthetic",
			Self::ProductionPrivate => "baseline-production-private",
			Self::ProductionPrivateAddendum => "baseline-production-private-addendum",
			Self::Soak => "baseline-soak-docker",
		}
	}
}

#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab")]
enum DiagnosticsCommand {
	/// Rebuild Qdrant from Postgres vectors through the admin API.
	QdrantRebuild(AdminPostArgs),
	/// Run raw admin search and include trace/result/source_ref data.
	RawSearch(AdminSearchArgs),
	/// List recent persisted search traces.
	RecentTraces(RecentTracesArgs),
	/// Read a bounded or full trace bundle.
	TraceBundle(TraceBundleArgs),
	/// Read note provenance, ingest decisions, outbox rows, and recent traces.
	NoteProvenance(NoteProvenanceArgs),
}

fn run_backfill(args: BackfillArgs) -> Result<()> {
	let task = if args.hundred_k {
		"baseline-backfill-100k-docker"
	} else if args.ten_k {
		"baseline-backfill-10k-docker"
	} else {
		"baseline-backfill-docker"
	};
	let mut env = BTreeMap::new();

	if let Some(docs) = args.docs {
		env.insert("ELF_BASELINE_BACKFILL_DOCS".to_string(), docs.to_string());
	}
	if let Some(worker_concurrency) = args.worker_concurrency {
		env.insert("ELF_BASELINE_WORKER_CONCURRENCY".to_string(), worker_concurrency.to_string());
	}

	if args.enable_expensive {
		env.insert("ELF_BASELINE_ENABLE_EXPENSIVE".to_string(), "1".to_string());
	}

	run_cargo_make("elf.cli.backfill/v1", task, env, args.dry_run, args.output.pretty)
}

fn run_benchmark(args: BenchmarkArgs) -> Result<()> {
	match args.command {
		BenchmarkCommand::Run(args) => run_benchmark_run(args),
		BenchmarkCommand::Report(args) => run_benchmark_report(args),
	}
}

fn run_benchmark_run(args: BenchmarkRunArgs) -> Result<()> {
	let task = args.kind.task_name();
	let mut env = BTreeMap::new();

	if let Some(projects) = args.projects {
		env.insert("ELF_BASELINE_PROJECTS".to_string(), projects);
	}
	if let Some(profile) = args.profile {
		env.insert("ELF_BASELINE_PROFILE".to_string(), profile);
	}
	if let Some(path) = args.production_corpus_manifest {
		env.insert("ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST".to_string(), path_display(&path));
	}
	if let Some(path) = args.private_addendum {
		env.insert("ELF_BASELINE_PRIVATE_ADDENDUM".to_string(), path_display(&path));
	}
	if let Some(seconds) = args.soak_seconds {
		env.insert("ELF_BASELINE_SOAK_SECONDS".to_string(), seconds.to_string());
	}

	run_cargo_make("elf.cli.benchmark_run/v1", task, env, args.dry_run, args.output.pretty)
}

fn run_benchmark_report(args: BenchmarkReportArgs) -> Result<()> {
	let mut env = BTreeMap::new();

	if let Some(path) = args.report {
		env.insert("ELF_BASELINE_REPORT".to_string(), path_display(&path));
	}
	if let Some(path) = args.out {
		env.insert("ELF_BASELINE_MARKDOWN_REPORT".to_string(), path_display(&path));
	}

	run_cargo_make(
		"elf.cli.benchmark_report/v1",
		"baseline-live-report",
		env,
		args.dry_run,
		args.output.pretty,
	)
}

fn search_body(
	query: String,
	mode: SearchMode,
	top_k: Option<u32>,
	candidate_k: Option<u32>,
	payload_level: PayloadLevel,
	filter_json: Option<&str>,
) -> Result<Value> {
	let mut body = serde_json::json!({
		"mode": mode.as_str(),
		"query": query,
		"top_k": top_k,
		"candidate_k": candidate_k,
		"payload_level": payload_level.as_str(),
	});

	if let Some(filter_json) = filter_json {
		body["filter"] = parse_json_object(filter_json, "--filter-json")?;
	}

	Ok(body)
}

fn source_ref(source_id: &Option<String>, source_ref_json: Option<&str>) -> Result<Value> {
	if let Some(source_ref_json) = source_ref_json {
		return parse_json_object(source_ref_json, "--source-ref-json");
	}

	Ok(source_id.as_ref().map_or_else(
		|| serde_json::json!({}),
		|source_id| serde_json::json!({"schema": "elf_cli/v1", "ref": {"source_id": source_id}}),
	))
}

fn parse_json_object(raw: &str, flag: &str) -> Result<Value> {
	let value: Value =
		serde_json::from_str(raw).map_err(|err| eyre::eyre!("{flag} must be valid JSON: {err}"))?;

	if !value.is_object() {
		return Err(eyre::eyre!("{flag} must be a JSON object."));
	}

	Ok(value)
}

fn add_context_headers(request: RequestBuilder, context: &ContextArgs) -> RequestBuilder {
	request
		.header("X-ELF-Tenant-Id", &context.tenant_id)
		.header("X-ELF-Project-Id", &context.project_id)
		.header("X-ELF-Agent-Id", &context.agent_id)
}

fn run_cargo_make(
	schema: &str,
	task: &str,
	env: BTreeMap<String, String>,
	dry_run: bool,
	pretty: bool,
) -> Result<()> {
	let command = serde_json::json!({
		"program": "cargo",
		"args": ["make", task],
		"env": env,
	});

	if dry_run {
		let output = serde_json::json!({
			"schema": schema,
			"dry_run": true,
			"command": command,
		});

		return write_json(&output, pretty);
	}

	let output = Command::new("cargo").arg("make").arg(task).envs(env.iter()).output()?;

	io::stderr().write_all(&output.stdout)?;
	io::stderr().write_all(&output.stderr)?;

	let status_code = output.status.code();
	let summary = serde_json::json!({
		"schema": schema,
		"dry_run": false,
		"command": command,
		"status_code": status_code,
		"success": output.status.success(),
	});

	write_json(&summary, pretty)?;

	if output.status.success() {
		Ok(())
	} else {
		Err(eyre::eyre!("cargo make {task} failed with status {status_code:?}."))
	}
}

fn write_json(value: &Value, pretty: bool) -> Result<()> {
	if pretty {
		serde_json::to_writer_pretty(io::stdout(), value)?;
	} else {
		serde_json::to_writer(io::stdout(), value)?;
	}

	writeln!(io::stdout())?;

	Ok(())
}

fn join_url(base_url: &str, path: &str) -> String {
	format!("{}/{}", base_url.trim_end_matches('/'), path.trim_start_matches('/'))
}

fn redact_url(url: &str) -> String {
	url.to_string()
}

fn header_string(headers: &HeaderMap, name: &str) -> Option<String> {
	headers.get(name).and_then(|value| value.to_str().ok()).map(str::to_string)
}

fn path_display(path: &Path) -> String {
	path.display().to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;

	run(Cli::parse()).await
}

async fn run(cli: Cli) -> Result<()> {
	let client = Client::new();

	match cli.command {
		Commands::AddNote(args) => run_add_note(&client, args).await,
		Commands::Search(args) => run_search(&client, args).await,
		Commands::Status(args) => run_status(&client, args).await,
		Commands::Backfill(args) => run_backfill(args),
		Commands::Benchmark(args) => run_benchmark(args),
		Commands::Diagnostics(args) => run_diagnostics(&client, args).await,
	}
}

async fn run_add_note(client: &Client, args: AddNoteArgs) -> Result<()> {
	let source_ref = source_ref(&args.source_id, args.source_ref_json.as_deref())?;
	let body = serde_json::json!({
		"scope": args.scope,
		"notes": [{
			"type": args.note_type,
			"key": args.key,
			"text": args.text,
			"importance": args.importance,
			"confidence": args.confidence,
			"ttl_days": args.ttl_days,
			"source_ref": source_ref,
		}],
	});
	let response = request_json(
		client,
		JsonRequest {
			method: Method::POST,
			base_url: &args.endpoint.api_url,
			path: "/v2/notes/ingest",
			token: args.endpoint.token.as_deref(),
			context: Some(&args.context),
			read_profile: None,
			body: Some(&body),
		},
	)
	.await?;
	let output = serde_json::json!({
		"schema": "elf.cli.add_note/v1",
		"request": {
			"api_url": redact_url(&args.endpoint.api_url),
			"tenant_id": args.context.tenant_id,
			"project_id": args.context.project_id,
			"agent_id": args.context.agent_id,
			"scope": body["scope"],
			"source_id": args.source_id,
			"source_ref": body["notes"][0]["source_ref"],
		},
		"response": response,
	});

	write_json(&output, args.output.pretty)
}

async fn run_search(client: &Client, args: SearchArgs) -> Result<()> {
	let body = search_body(
		args.query,
		args.mode,
		args.top_k,
		args.candidate_k,
		args.payload_level,
		args.filter_json.as_deref(),
	)?;
	let response = request_json(
		client,
		JsonRequest {
			method: Method::POST,
			base_url: &args.endpoint.api_url,
			path: "/v2/searches",
			token: args.endpoint.token.as_deref(),
			context: Some(&args.read_context.context),
			read_profile: Some(&args.read_context.read_profile),
			body: Some(&body),
		},
	)
	.await?;
	let output = serde_json::json!({
		"schema": "elf.cli.search/v1",
		"request": {
			"api_url": redact_url(&args.endpoint.api_url),
			"tenant_id": args.read_context.context.tenant_id,
			"project_id": args.read_context.context.project_id,
			"agent_id": args.read_context.context.agent_id,
			"read_profile": args.read_context.read_profile,
			"mode": body["mode"],
			"payload_level": body["payload_level"],
		},
		"trace_id": response.get("trace_id").cloned().unwrap_or(Value::Null),
		"search_id": response.get("search_id").cloned().unwrap_or(Value::Null),
		"response": response,
	});

	write_json(&output, args.output.pretty)
}

async fn run_status(client: &Client, args: StatusArgs) -> Result<()> {
	let url = join_url(&args.endpoint.api_url, "/health");
	let mut request = client.get(&url);

	if let Some(token) = args.endpoint.token.as_deref() {
		request = request.bearer_auth(token);
	}

	let response = request.send().await?;
	let status = response.status();
	let request_id = header_string(response.headers(), "x-elf-request-id");
	let body = response.text().await?;
	let output = serde_json::json!({
		"schema": "elf.cli.status/v1",
		"api": {
			"url": redact_url(&args.endpoint.api_url),
			"healthy": status == StatusCode::OK,
			"status": status.as_u16(),
			"request_id": request_id,
			"body": body,
		},
	});

	write_json(&output, args.output.pretty)?;

	if status.is_success() {
		Ok(())
	} else {
		Err(eyre::eyre!("ELF API health check failed with HTTP status {status}."))
	}
}

async fn run_diagnostics(client: &Client, args: DiagnosticsArgs) -> Result<()> {
	match args.command {
		DiagnosticsCommand::QdrantRebuild(args) => run_qdrant_rebuild(client, args).await,
		DiagnosticsCommand::RawSearch(args) => run_raw_search(client, args).await,
		DiagnosticsCommand::RecentTraces(args) => run_recent_traces(client, args).await,
		DiagnosticsCommand::TraceBundle(args) => run_trace_bundle(client, args).await,
		DiagnosticsCommand::NoteProvenance(args) => run_note_provenance(client, args).await,
	}
}

async fn run_qdrant_rebuild(client: &Client, args: AdminPostArgs) -> Result<()> {
	let response = request_json(
		client,
		JsonRequest {
			method: Method::POST,
			base_url: &args.endpoint.admin_url,
			path: "/v2/admin/qdrant/rebuild",
			token: args.endpoint.admin_token.as_deref(),
			context: Some(&args.context),
			read_profile: None,
			body: None,
		},
	)
	.await?;
	let output = serde_json::json!({
		"schema": "elf.cli.diagnostics.qdrant_rebuild/v1",
		"admin_url": redact_url(&args.endpoint.admin_url),
		"response": response,
	});

	write_json(&output, args.output.pretty)
}

async fn run_raw_search(client: &Client, args: AdminSearchArgs) -> Result<()> {
	let body = search_body(
		args.query,
		args.mode,
		args.top_k,
		args.candidate_k,
		args.payload_level,
		args.filter_json.as_deref(),
	)?;
	let response = request_json(
		client,
		JsonRequest {
			method: Method::POST,
			base_url: &args.endpoint.admin_url,
			path: "/v2/admin/searches/raw",
			token: args.endpoint.admin_token.as_deref(),
			context: Some(&args.read_context.context),
			read_profile: Some(&args.read_context.read_profile),
			body: Some(&body),
		},
	)
	.await?;
	let output = serde_json::json!({
		"schema": "elf.cli.diagnostics.raw_search/v1",
		"request": {
			"admin_url": redact_url(&args.endpoint.admin_url),
			"tenant_id": args.read_context.context.tenant_id,
			"project_id": args.read_context.context.project_id,
			"agent_id": args.read_context.context.agent_id,
			"read_profile": args.read_context.read_profile,
			"mode": body["mode"],
			"payload_level": body["payload_level"],
		},
		"trace_id": response.get("trace_id").cloned().unwrap_or(Value::Null),
		"response": response,
	});

	write_json(&output, args.output.pretty)
}

async fn run_recent_traces(client: &Client, args: RecentTracesArgs) -> Result<()> {
	let mut query = Vec::new();

	if let Some(limit) = args.limit {
		query.push(("limit", limit.to_string()));
	}

	let response = request_json_query(
		client,
		&args.endpoint.admin_url,
		"/v2/admin/traces/recent",
		args.endpoint.admin_token.as_deref(),
		&args.context,
		&query,
	)
	.await?;
	let output = serde_json::json!({
		"schema": "elf.cli.diagnostics.recent_traces/v1",
		"admin_url": redact_url(&args.endpoint.admin_url),
		"response": response,
	});

	write_json(&output, args.output.pretty)
}

async fn run_trace_bundle(client: &Client, args: TraceBundleArgs) -> Result<()> {
	let path = format!("/v2/admin/traces/{}/bundle", args.trace_id);
	let mut query = vec![("mode", args.mode)];

	if let Some(limit) = args.stage_items_limit {
		query.push(("stage_items_limit", limit.to_string()));
	}
	if let Some(limit) = args.candidates_limit {
		query.push(("candidates_limit", limit.to_string()));
	}

	let response = request_json_query(
		client,
		&args.endpoint.admin_url,
		&path,
		args.endpoint.admin_token.as_deref(),
		&args.context,
		&query,
	)
	.await?;
	let output = serde_json::json!({
		"schema": "elf.cli.diagnostics.trace_bundle/v1",
		"admin_url": redact_url(&args.endpoint.admin_url),
		"trace_id": response.pointer("/trace/trace_id").cloned().unwrap_or(Value::Null),
		"response": response,
	});

	write_json(&output, args.output.pretty)
}

async fn run_note_provenance(client: &Client, args: NoteProvenanceArgs) -> Result<()> {
	let path = format!("/v2/admin/notes/{}/provenance", args.note_id);
	let response = request_json_query(
		client,
		&args.endpoint.admin_url,
		&path,
		args.endpoint.admin_token.as_deref(),
		&args.context,
		&[],
	)
	.await?;
	let output = serde_json::json!({
		"schema": "elf.cli.diagnostics.note_provenance/v1",
		"admin_url": redact_url(&args.endpoint.admin_url),
		"note_id": response.pointer("/note/note_id").cloned().unwrap_or(Value::String(args.note_id)),
		"response": response,
	});

	write_json(&output, args.output.pretty)
}

async fn request_json(client: &Client, args: JsonRequest<'_>) -> Result<Value> {
	let mut request = client.request(args.method, join_url(args.base_url, args.path));

	if let Some(token) = args.token {
		request = request.bearer_auth(token);
	}
	if let Some(context) = args.context {
		request = add_context_headers(request, context);
	}
	if let Some(read_profile) = args.read_profile {
		request = request.header("X-ELF-Read-Profile", read_profile);
	}
	if let Some(body) = args.body {
		request = request.json(body);
	}

	parse_json_response(request.send().await?).await
}

async fn request_json_query(
	client: &Client,
	base_url: &str,
	path: &str,
	token: Option<&str>,
	context: &ContextArgs,
	query: &[(&str, String)],
) -> Result<Value> {
	let mut request = client.get(join_url(base_url, path)).query(query);

	if let Some(token) = token {
		request = request.bearer_auth(token);
	}

	request = add_context_headers(request, context);

	parse_json_response(request.send().await?).await
}

async fn parse_json_response(response: Response) -> Result<Value> {
	let status = response.status();
	let request_id = header_string(response.headers(), "x-elf-request-id");
	let text = response.text().await?;

	if !status.is_success() {
		return Err(eyre::eyre!(
			"ELF request failed with HTTP status {status} and request_id {}: {text}",
			request_id.as_deref().unwrap_or("unknown")
		));
	}
	if text.trim().is_empty() {
		return Ok(serde_json::json!({"status": status.as_u16(), "request_id": request_id}));
	}

	serde_json::from_str(&text).map_err(|err| {
		eyre::eyre!(
			"ELF response was not valid JSON for request_id {}: {err}",
			request_id.as_deref().unwrap_or("unknown")
		)
	})
}
