use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

use crate::args::OutputArgs;

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
