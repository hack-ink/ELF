use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

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
pub(crate) struct Cli {
	#[command(subcommand)]
	pub(crate) command: Commands,
}

#[derive(Debug, Args)]
pub(crate) struct PublicEndpointArgs {
	/// Public ELF API base URL.
	#[arg(long, env = "ELF_API_URL", default_value = DEFAULT_API_URL)]
	pub(crate) api_url: String,
	/// Optional bearer token for static-key auth.
	#[arg(long, env = "ELF_USER_TOKEN")]
	pub(crate) token: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct AdminEndpointArgs {
	/// Admin ELF API base URL.
	#[arg(long, env = "ELF_ADMIN_URL", default_value = DEFAULT_ADMIN_URL)]
	pub(crate) admin_url: String,
	/// Optional admin bearer token for static-key auth.
	#[arg(long, env = "ELF_ADMIN_TOKEN")]
	pub(crate) admin_token: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub(crate) struct ContextArgs {
	/// Tenant id sent in X-ELF-Tenant-Id.
	#[arg(long, env = "ELF_TENANT_ID", default_value = DEFAULT_TENANT_ID)]
	pub(crate) tenant_id: String,
	/// Project id sent in X-ELF-Project-Id.
	#[arg(long, env = "ELF_PROJECT_ID", default_value = DEFAULT_PROJECT_ID)]
	pub(crate) project_id: String,
	/// Agent id sent in X-ELF-Agent-Id.
	#[arg(long, env = "ELF_AGENT_ID", default_value = DEFAULT_AGENT_ID)]
	pub(crate) agent_id: String,
}

#[derive(Clone, Debug, Args)]
pub(crate) struct ReadContextArgs {
	#[command(flatten)]
	pub(crate) context: ContextArgs,
	/// Read profile sent in X-ELF-Read-Profile.
	#[arg(long, env = "ELF_READ_PROFILE", default_value = DEFAULT_READ_PROFILE)]
	pub(crate) read_profile: String,
}

#[derive(Debug, Args)]
pub(crate) struct OutputArgs {
	/// Pretty-print the JSON output.
	#[arg(long)]
	pub(crate) pretty: bool,
}

#[derive(Debug, Args)]
pub(crate) struct AddNoteArgs {
	#[command(flatten)]
	pub(crate) endpoint: PublicEndpointArgs,
	#[command(flatten)]
	pub(crate) context: ContextArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// Scope applied to the note.
	#[arg(long, default_value = "agent_private")]
	pub(crate) scope: String,
	/// Memory note type.
	#[arg(long = "type", default_value = "fact")]
	pub(crate) note_type: String,
	/// Optional note key used by the update resolver.
	#[arg(long)]
	pub(crate) key: Option<String>,
	/// English note text.
	#[arg(long)]
	pub(crate) text: String,
	/// Ranking importance value.
	#[arg(long, default_value_t = 0.7)]
	pub(crate) importance: f32,
	/// Ranking confidence value.
	#[arg(long, default_value_t = 0.9)]
	pub(crate) confidence: f32,
	/// Optional TTL override in days.
	#[arg(long)]
	pub(crate) ttl_days: Option<i64>,
	/// Operator-visible source id copied into source_ref.ref.source_id.
	#[arg(long)]
	pub(crate) source_id: Option<String>,
	/// Full JSON object source_ref override.
	#[arg(long)]
	pub(crate) source_ref_json: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct SearchArgs {
	#[command(flatten)]
	pub(crate) endpoint: PublicEndpointArgs,
	#[command(flatten)]
	pub(crate) read_context: ReadContextArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// English query string.
	#[arg(long)]
	pub(crate) query: String,
	/// Search mode to request from the service.
	#[arg(long, value_enum, default_value_t = SearchMode::QuickFind)]
	pub(crate) mode: SearchMode,
	/// Number of final items to return.
	#[arg(long)]
	pub(crate) top_k: Option<u32>,
	/// Candidate breadth before ranking.
	#[arg(long)]
	pub(crate) candidate_k: Option<u32>,
	/// Payload level requested from the service.
	#[arg(long, value_enum, default_value_t = PayloadLevel::L0)]
	pub(crate) payload_level: PayloadLevel,
	/// Optional search filter JSON object.
	#[arg(long)]
	pub(crate) filter_json: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct StatusArgs {
	#[command(flatten)]
	pub(crate) endpoint: PublicEndpointArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(crate) struct BackfillArgs {
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// Backfill corpus document count override.
	#[arg(long)]
	pub(crate) docs: Option<u32>,
	/// Worker concurrency override for the backfill runner.
	#[arg(long)]
	pub(crate) worker_concurrency: Option<u32>,
	/// Use the checked-in 10k operator profile task.
	#[arg(long)]
	pub(crate) ten_k: bool,
	/// Use the guarded 100k operator profile task.
	#[arg(long, conflicts_with = "ten_k")]
	pub(crate) hundred_k: bool,
	/// Set the required expensive-run guard for the 100k task.
	#[arg(long)]
	pub(crate) enable_expensive: bool,
	/// Print the resolved task and environment without running it.
	#[arg(long)]
	pub(crate) dry_run: bool,
}

#[derive(Debug, Args)]
pub(crate) struct BenchmarkArgs {
	#[command(subcommand)]
	pub(crate) command: BenchmarkCommand,
}

#[derive(Debug, Args)]
pub(crate) struct BenchmarkRunArgs {
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// Benchmark task wrapper to run.
	#[arg(long, value_enum, default_value_t = BenchmarkRunKind::Live)]
	pub(crate) kind: BenchmarkRunKind,
	/// Project filter passed to ELF_BASELINE_PROJECTS.
	#[arg(long)]
	pub(crate) projects: Option<String>,
	/// Corpus profile passed to ELF_BASELINE_PROFILE.
	#[arg(long)]
	pub(crate) profile: Option<String>,
	/// Private production corpus manifest path.
	#[arg(long)]
	pub(crate) production_corpus_manifest: Option<PathBuf>,
	/// Markdown addendum path for production-private-addendum.
	#[arg(long)]
	pub(crate) private_addendum: Option<PathBuf>,
	/// Soak duration override in seconds.
	#[arg(long)]
	pub(crate) soak_seconds: Option<u32>,
	/// Print the resolved task and environment without running it.
	#[arg(long)]
	pub(crate) dry_run: bool,
}

#[derive(Debug, Args)]
pub(crate) struct BenchmarkReportArgs {
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// Source live-baseline report JSON path.
	#[arg(long)]
	pub(crate) report: Option<PathBuf>,
	/// Markdown output path.
	#[arg(long)]
	pub(crate) out: Option<PathBuf>,
	/// Print the resolved task and environment without running it.
	#[arg(long)]
	pub(crate) dry_run: bool,
}

#[derive(Debug, Args)]
pub(crate) struct DiagnosticsArgs {
	#[command(subcommand)]
	pub(crate) command: DiagnosticsCommand,
}

#[derive(Debug, Args)]
pub(crate) struct AdminPostArgs {
	#[command(flatten)]
	pub(crate) endpoint: AdminEndpointArgs,
	#[command(flatten)]
	pub(crate) context: ContextArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
}

#[derive(Debug, Args)]
pub(crate) struct AdminSearchArgs {
	#[command(flatten)]
	pub(crate) endpoint: AdminEndpointArgs,
	#[command(flatten)]
	pub(crate) read_context: ReadContextArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// English query string.
	#[arg(long)]
	pub(crate) query: String,
	/// Search mode to request from the service.
	#[arg(long, value_enum, default_value_t = SearchMode::QuickFind)]
	pub(crate) mode: SearchMode,
	/// Number of final items to return.
	#[arg(long)]
	pub(crate) top_k: Option<u32>,
	/// Candidate breadth before ranking.
	#[arg(long)]
	pub(crate) candidate_k: Option<u32>,
	/// Payload level requested from the service.
	#[arg(long, value_enum, default_value_t = PayloadLevel::L2)]
	pub(crate) payload_level: PayloadLevel,
	/// Optional search filter JSON object.
	#[arg(long)]
	pub(crate) filter_json: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct RecentTracesArgs {
	#[command(flatten)]
	pub(crate) endpoint: AdminEndpointArgs,
	#[command(flatten)]
	pub(crate) context: ContextArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// Maximum trace headers to return.
	#[arg(long)]
	pub(crate) limit: Option<u32>,
}

#[derive(Debug, Args)]
pub(crate) struct TraceBundleArgs {
	#[command(flatten)]
	pub(crate) endpoint: AdminEndpointArgs,
	#[command(flatten)]
	pub(crate) context: ContextArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// Trace id to load.
	#[arg(long)]
	pub(crate) trace_id: String,
	/// Bundle mode: bounded or full.
	#[arg(long, default_value = "bounded")]
	pub(crate) mode: String,
	/// Optional per-stage item cap.
	#[arg(long)]
	pub(crate) stage_items_limit: Option<u32>,
	/// Optional replay candidate cap.
	#[arg(long)]
	pub(crate) candidates_limit: Option<u32>,
}

#[derive(Debug, Args)]
pub(crate) struct NoteProvenanceArgs {
	#[command(flatten)]
	pub(crate) endpoint: AdminEndpointArgs,
	#[command(flatten)]
	pub(crate) context: ContextArgs,
	#[command(flatten)]
	pub(crate) output: OutputArgs,
	/// Note id to inspect.
	#[arg(long)]
	pub(crate) note_id: String,
}

#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab")]
pub(crate) enum Commands {
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
pub(crate) enum SearchMode {
	QuickFind,
	PlannedSearch,
}

impl SearchMode {
	pub(crate) fn as_str(self) -> &'static str {
		match self {
			Self::QuickFind => "quick_find",
			Self::PlannedSearch => "planned_search",
		}
	}
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "lower")]
pub(crate) enum PayloadLevel {
	L0,
	L1,
	L2,
}

impl PayloadLevel {
	pub(crate) fn as_str(self) -> &'static str {
		match self {
			Self::L0 => "l0",
			Self::L1 => "l1",
			Self::L2 => "l2",
		}
	}
}

#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab")]
pub(crate) enum BenchmarkCommand {
	/// Run one checked-in Docker baseline task.
	Run(BenchmarkRunArgs),
	/// Render Markdown from a live-baseline JSON report.
	Report(BenchmarkReportArgs),
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab")]
pub(crate) enum BenchmarkRunKind {
	Live,
	ProductionSynthetic,
	ProductionPrivate,
	ProductionPrivateAddendum,
	Soak,
}

impl BenchmarkRunKind {
	pub(crate) fn task_name(self) -> &'static str {
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
pub(crate) enum DiagnosticsCommand {
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
