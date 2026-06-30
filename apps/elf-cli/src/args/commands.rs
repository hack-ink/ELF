use clap::{Parser, Subcommand};

use crate::args::{
	AddNoteArgs, BackfillArgs, BenchmarkArgs, DiagnosticsArgs, SearchArgs, StatusArgs,
};

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
