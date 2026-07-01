use crate::{
	DEFAULT_ADAPTER_BEHAVIOR, DEFAULT_ADAPTER_ID, DEFAULT_ADAPTER_NAME, DEFAULT_ADAPTER_NOTES,
	DEFAULT_ADAPTER_RUNTIME_STATUS, DEFAULT_ADAPTER_STORAGE_STATUS,
	DEFAULT_EXTERNAL_ADAPTER_MANIFEST_PATH, DEFAULT_FIXTURE_PATH, DEFAULT_MARKDOWN_PATH,
	DEFAULT_REPORT_PATH, DEFAULT_RUN_ID, Parser, PathBuf, Subcommand,
};

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
	/// Optional same-corpus quantitative product manifest to merge into the report.
	#[arg(long, value_name = "FILE")]
	pub(super) quantitative_product_manifest: Option<PathBuf>,
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

#[derive(Debug, Parser)]
pub(super) struct ExportQuantitativeProductManifestArgs {
	/// Generated real_world_job JSON report to export.
	#[arg(long, value_name = "FILE", default_value = DEFAULT_REPORT_PATH)]
	pub(super) report: PathBuf,
	/// Write product manifest JSON to this file. Omit to print to stdout.
	#[arg(long, value_name = "FILE")]
	pub(super) out: Option<PathBuf>,
	/// Stable manifest id. Defaults to <run_id>-quantitative-product-manifest.
	#[arg(long)]
	pub(super) manifest_id: Option<String>,
	/// Override the exported product name.
	#[arg(long)]
	pub(super) product: Option<String>,
	/// Override the exported adapter id.
	#[arg(long)]
	pub(super) adapter_id: Option<String>,
	/// Override the exported adapter name.
	#[arg(long)]
	pub(super) adapter_name: Option<String>,
}

#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab")]
pub(super) enum Command {
	/// Export the primary quantitative row as a reusable product manifest.
	ExportQuantitativeProductManifest(ExportQuantitativeProductManifestArgs),
	/// Parse and score real_world_job fixtures, then emit a JSON report.
	Run(RunArgs),
	/// Render Markdown from a generated real_world_job JSON report.
	Publish(PublishArgs),
}
