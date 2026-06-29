use super::*;

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
pub(super) struct Args {
	#[command(subcommand)]
	pub(super) command: Command,
}

#[derive(Debug, Parser)]
pub(super) struct RunArgs {
	/// Fixture file or directory containing real_world_job JSON fixtures.
	#[arg(long, value_name = "PATH", default_value = DEFAULT_FIXTURE_PATH)]
	pub(super) fixtures: PathBuf,
	/// Write report JSON to this file. Omit to print to stdout.
	#[arg(long, value_name = "FILE")]
	pub(super) out: Option<PathBuf>,
	/// Stable run id recorded in the generated report.
	#[arg(long, default_value = DEFAULT_RUN_ID)]
	pub(super) run_id: String,
	/// Adapter id recorded for the offline smoke response.
	#[arg(long, default_value = DEFAULT_ADAPTER_ID)]
	pub(super) adapter_id: String,
	/// Human-readable adapter name recorded in the generated report.
	#[arg(long, default_value = DEFAULT_ADAPTER_NAME)]
	pub(super) adapter_name: String,
	/// Adapter behavior label recorded in the generated report.
	#[arg(long, default_value = DEFAULT_ADAPTER_BEHAVIOR)]
	pub(super) adapter_behavior: String,
	/// Adapter storage typed status recorded in the generated report.
	#[arg(long, default_value = DEFAULT_ADAPTER_STORAGE_STATUS)]
	pub(super) adapter_storage_status: String,
	/// Adapter runtime typed status recorded in the generated report.
	#[arg(long, default_value = DEFAULT_ADAPTER_RUNTIME_STATUS)]
	pub(super) adapter_runtime_status: String,
	/// Adapter notes recorded in the generated report.
	#[arg(long, default_value = DEFAULT_ADAPTER_NOTES)]
	pub(super) adapter_notes: String,
	/// Real-world external adapter manifest to include in report coverage.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_EXTERNAL_ADAPTER_MANIFEST_PATH)]
	pub(super) external_adapter_manifest: PathBuf,
	/// Skip loading the real-world external adapter coverage manifest.
	#[arg(long)]
	pub(super) skip_external_adapter_manifest: bool,
}

#[derive(Debug, Parser)]
pub(super) struct PublishArgs {
	/// Generated real_world_job JSON report.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_REPORT_PATH)]
	pub(super) report: PathBuf,
	/// Write Markdown to this file. Omit to print to stdout.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_MARKDOWN_PATH)]
	pub(super) out: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab")]
pub(super) enum Command {
	/// Parse and score real_world_job fixtures, then emit a JSON report.
	Run(RunArgs),
	/// Render Markdown from a generated real_world_job JSON report.
	Publish(PublishArgs),
}
